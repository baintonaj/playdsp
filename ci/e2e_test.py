#!/usr/bin/env python3
"""End-to-end tests for playdsp. Standard library only, runs on macOS, Linux and Windows.

Usage:  cargo build --release && python3 ci/e2e_test.py [--keep]

Each scenario creates a fresh project with `playdsp new`, writes WAV files,
runs the real binary, and checks the rendered output sample by sample.
Set PLAYDSP_REQUIRE_AVX=1 on x86_64 machines to make the runtime self-tests
fail (instead of skip) if the AVX conversion path cannot run.
"""

import os
import random
import re
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
EXE = ".exe" if os.name == "nt" else ""
PLAYDSP = REPO / "target" / "release" / f"playdsp{EXE}"
GAIN = 10 ** (-12 / 20)  # starter code applies -12 dB


# ---------------------------------------------------------------------------
# WAV helpers
# ---------------------------------------------------------------------------

def quantise(x, kind, bits):
    """The value the runtime should decode for sample x stored as kind/bits."""
    if kind == "float":
        return struct.unpack("<f", struct.pack("<f", x))[0] if bits == 32 else x
    scale = 2 ** (bits - 1)
    return max(-scale, min(scale - 1, round(x * scale))) / scale


def write_wav(path, frames, sample_rate, kind, bits):
    """frames: list of per-frame tuples of floats in [-1, 1)."""
    channels = len(frames[0])
    flat = [s for frame in frames for s in frame]
    if kind == "float":
        raw = struct.pack(f"<{len(flat)}{'f' if bits == 32 else 'd'}", *flat)
        tag = 3
    else:
        tag = 1
        ints = [round(quantise(s, kind, bits) * 2 ** (bits - 1)) for s in flat]
        if bits == 8:
            raw = bytes(v + 128 for v in ints)  # 8-bit WAV is unsigned
        elif bits == 16:
            raw = struct.pack(f"<{len(ints)}h", *ints)
        elif bits == 24:
            raw = b"".join(struct.pack("<i", v)[:3] for v in ints)
        else:
            raw = struct.pack(f"<{len(ints)}i", *ints)
    block_align = channels * bits // 8
    fmt = struct.pack("<HHIIHH", tag, channels, sample_rate, sample_rate * block_align, block_align, bits)
    body = b"WAVE" + b"fmt " + struct.pack("<I", len(fmt)) + fmt + b"data" + struct.pack("<I", len(raw)) + raw
    if len(raw) % 2:
        body += b"\0"
    Path(path).write_bytes(b"RIFF" + struct.pack("<I", len(body)) + body)


def read_float_wav(path):
    """Returns (channels as lists, sample_rate) for a 32-bit float WAV."""
    data = Path(path).read_bytes()
    pos, fmt = 12, None
    while pos + 8 <= len(data):
        chunk_id = data[pos:pos + 4]
        size = struct.unpack("<I", data[pos + 4:pos + 8])[0]
        body = data[pos + 8:pos + 8 + size]
        if chunk_id == b"fmt ":
            fmt = struct.unpack("<HHIIHH", body[:16])
        elif chunk_id == b"data":
            tag, channels, sample_rate, _, _, bits = fmt
            assert (tag, bits) == (3, 32), f"{path}: expected 32-bit float output, got tag {tag}, {bits} bits"
            flat = struct.unpack(f"<{len(body) // 4}f", body)
            return [list(flat[c::channels]) for c in range(channels)], sample_rate
        pos += 8 + size + (size & 1)
    raise AssertionError(f"{path}: no data chunk")


# ---------------------------------------------------------------------------
# Test harness
# ---------------------------------------------------------------------------

FAILURES = []


def check(condition, message):
    print(("  ok    " if condition else "  FAIL  ") + message)
    if not condition:
        FAILURES.append(message)


def run(args, cwd, env=None, expect_ok=True):
    result = subprocess.run(
        args, cwd=cwd, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace"
    )
    output = result.stdout + result.stderr
    if expect_ok and result.returncode != 0:
        print(output)
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(map(str, args))}")
    return output


def new_project(root, name):
    base = root / name
    run([PLAYDSP, "new", "--dir", base], cwd=root)
    return base / "audio"


def playdsp(audio_dir, *args, expect_ok=True):
    return run([PLAYDSP, *args], cwd=audio_dir, expect_ok=expect_ok)


def results(audio_dir, pattern="*"):
    return sorted((audio_dir / "result").glob(pattern))


def noise(frames, channels, seed, level=0.5):
    rng = random.Random(seed)
    return [tuple(rng.uniform(-level, level) for _ in range(channels)) for _ in range(frames)]


def max_abs_diff(a, b):
    return max((abs(x - y) for x, y in zip(a, b)), default=0.0)


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------

FORMATS = [("int", 8), ("int", 16), ("int", 24), ("int", 32), ("float", 32), ("float", 64)]


def test_formats_and_null(root):
    print("\n[formats] every input format, Rust vs C++ null test")
    audio = new_project(root, "formats")
    sr, frames = 44100, noise(44100, 2, seed=1)
    for kind, bits in FORMATS:
        write_wav(audio / "source" / f"s_{kind}{bits}.wav", frames, sr, kind, bits)
    write_wav(audio / "source" / "MULTI6.WAV", noise(3000, 6, seed=2), 48000, "int", 24)

    playdsp(audio)
    check(len(results(audio)) == 2 * (len(FORMATS) + 1), f"{len(results(audio))} output files (expected 14)")

    for kind, bits in FORMATS:
        stem = f"s_{kind}{bits}"
        expected = [[quantise(f[c], kind, bits) * GAIN for f in frames] for c in range(2)]
        rendered = {}
        for lang in ("rs", "cpp"):
            files = results(audio, f"{stem}_processed_*_{lang}.wav")
            if not files:
                check(False, f"{stem} {lang}: output missing")
                continue
            out, out_sr = read_float_wav(files[0])
            rendered[lang] = out
            err = max(max_abs_diff(out[c], expected[c]) for c in range(2))
            check(out_sr == sr and all(len(ch) == len(frames) for ch in out) and err < 1e-7,
                  f"{stem} {lang}: length {len(out[0])}, -12 dB error {err:.1e}")
        if len(rendered) == 2:
            null = max(max_abs_diff(rendered["rs"][c], rendered["cpp"][c]) for c in range(2))
            check(null == 0.0, f"{stem}: Rust vs C++ null = {null:.1e}")

    for lang in ("rs", "cpp"):
        files = results(audio, f"MULTI6_processed_*_{lang}.wav")
        out, out_sr = read_float_wav(files[0]) if files else ([], 0)
        check(len(out) == 6 and out_sr == 48000 and len(out[0]) == 3000,
              f"MULTI6.WAV {lang}: 6 channels @ 48 kHz, 3000 frames (uppercase extension)")
    return audio


def test_incremental_rebuild(audio):
    print("\n[rebuild] unchanged code must not relink the runtime")
    binary = audio / ".playdsp_runtime" / "target" / "release" / f"playdsp_runtime{EXE}"
    before = binary.stat().st_mtime_ns
    playdsp(audio, "--rust")
    check(binary.stat().st_mtime_ns == before, "runtime binary untouched by a no-change run")


def test_dsp_test_harness(audio):
    print("\n[playdsp test] starter Rust + C++ tests")
    out = playdsp(audio, "test")
    check("6 passed" in out and "test result: ok" in out, "6 starter tests pass")


def test_runtime_selftests(audio):
    print("\n[self-tests] AVX vs scalar conversion, tail detection")
    env = dict(os.environ)
    env["RUSTFLAGS"] = (env.get("RUSTFLAGS", "") + " --cfg playdsp_selftest").strip()
    out = run(["cargo", "test", "runtime_selftests", "--", "--nocapture"],
              cwd=audio / ".playdsp_runtime", env=env, expect_ok=False)
    passed = [int(n) for n in re.findall(r"test result: ok\. (\d+) passed", out)]
    # 6 portable tests, plus the AVX test on x86_64.
    check(sum(passed) >= 6 and "FAILED" not in out, f"runtime self-tests pass ({sum(passed)} run)")
    if os.environ.get("PLAYDSP_REQUIRE_AVX") == "1":
        check("AVX path verified" in out, "AVX conversion path exercised")
    if "FAILED" in out or "test result: ok" not in out:
        print(out)


def test_new_does_not_overwrite(root, audio):
    print("\n[new] re-running `playdsp new` keeps user files")
    rust_file = audio / "processing" / "rust" / "rust_process_audio.rs"
    rust_file.write_text(rust_file.read_text() + "\n// user edit\n")
    out = run([PLAYDSP, "new", "--dir", audio.parent], cwd=root)
    check(out.count("Skipped") == 4, "4 existing starter files skipped")
    check(rust_file.read_text().endswith("// user edit\n"), "user edit preserved")


def test_code_import(root):
    print("\n[-d] importing conventionally formatted code")
    audio = new_project(root, "import")
    donor = new_project(root, "import_donor") / "processing" / "rust"
    out = playdsp(audio, "-d", donor, "--rust")
    check("File copied to" in out and "Invalid" not in out, "starter-formatted rust_process_audio.rs accepted")


RUST_ECHO = """use std::sync::{LazyLock, Mutex};

// One echo at half level, 1.5 s later; delay length comes from the runtime sample rate.
struct State { line: Vec<Vec<f64>>, pos: usize }

impl State {
    fn process(&mut self, input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
        let d = (1.5 * crate::playdsp_sample_rate()) as usize;
        if self.line.len() < input.len() { self.line.resize(input.len(), vec![0.0; d]); }
        for i in 0..input[0].len() {
            for ch in 0..input.len() {
                let delayed = self.line[ch][self.pos];
                self.line[ch][self.pos] = input[ch][i];
                output[ch][i] = input[ch][i] + 0.5 * delayed;
            }
            self.pos = (self.pos + 1) % d;
        }
    }
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State { line: vec![], pos: 0 }));

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    STATE.lock().unwrap().process(input, output);
}
"""

CPP_LATENCY = """#include <cstddef>
#include <mutex>
#include <vector>

extern "C" void playdsp_set_latency(std::size_t samples);

// Pure 64-sample delay that reports its latency, so the output should line up with the input.
extern "C" void cpp_process(const double* input, size_t num_channels,
                            size_t num_samples, double* output) {
    static std::mutex m;
    static std::vector<std::vector<double>> line;
    static std::size_t pos = 0;
    const std::size_t latency = 64;
    std::scoped_lock lock(m);
    playdsp_set_latency(latency);
    if (line.size() < num_channels) line.resize(num_channels, std::vector<double>(latency, 0.0));
    for (std::size_t s = 0; s < num_samples; s++) {
        for (std::size_t c = 0; c < num_channels; c++) {
            output[s * num_channels + c] = line[c][pos];
            line[c][pos] = input[s * num_channels + c];
        }
        pos = (pos + 1) % latency;
    }
}
"""


def test_tail_latency_and_sample_rate(root):
    print("\n[tail/latency] late echo kept, reported latency compensated")
    audio = new_project(root, "delay")
    (audio / "processing" / "rust" / "rust_process_audio.rs").write_text(RUST_ECHO)
    (audio / "processing" / "cpp" / "cpp_process_audio.cpp").write_text(CPP_LATENCY)
    sr, burst = 44100, 4410
    frames = noise(burst, 2, seed=3) + [(0.0, 0.0)] * (sr - burst)
    frames = [tuple(quantise(s, "float", 32) for s in f) for f in frames]
    write_wav(audio / "source" / "burst.wav", frames, sr, "float", 32)
    playdsp(audio)

    out, _ = read_float_wav(results(audio, "*_rs.wav")[0])
    delay = int(1.5 * sr)
    echo_err = max(abs(out[c][delay + i] - 0.5 * frames[i][c]) for c in range(2) for i in range(burst))
    check(len(out[0]) >= delay + burst, f"Rust output {len(out[0]) / sr:.3f} s keeps the echo that starts after a 0.5 s gap")
    check(echo_err < 1e-7, f"echo at exactly 1.5 s via playdsp_sample_rate() (error {echo_err:.1e})")

    out, _ = read_float_wav(results(audio, "*_cpp.wav")[0])
    err = max(max_abs_diff(out[c], [f[c] for f in frames]) for c in range(2))
    check(len(out[0]) == sr and err == 0.0, f"C++ 64-sample delay aligned by playdsp_set_latency (error {err:.1e})")


RUST_RESIDUAL = """// Outputs the part of each sample an f32 cannot represent, scaled by 2^24.
pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    for (i, o) in input.iter().zip(output.iter_mut()) {
        for (x, y) in i.iter().zip(o.iter_mut()) {
            *y = (*x - (*x as f32) as f64) * 16777216.0;
        }
    }
}
"""


def test_rust_only_and_precision(root):
    print("\n[no C++] Rust-only project builds, tests skip C++, 32-bit int keeps full precision")
    audio = new_project(root, "rust_only")
    (audio / "processing" / "cpp" / "cpp_process_audio.cpp").unlink()
    (audio / "processing" / "rust" / "rust_process_audio.rs").write_text(RUST_RESIDUAL)
    write_wav(audio / "source" / "s_int32.wav", noise(8192, 2, seed=4), 44100, "int", 32)
    playdsp(audio)
    outputs = results(audio)
    check([p.name.endswith("_rs.wav") for p in outputs] == [True], "exactly one Rust output, no C++ attempted")
    if outputs:
        out, _ = read_float_wav(outputs[0])
        nonzero = sum(1 for ch in out for s in ch if s != 0.0) / (2 * 8192)
        check(nonzero > 0.5, f"sub-f32 detail survives decoding ({nonzero:.0%} of samples non-zero)")

    out = playdsp(audio, "test", expect_ok=False)
    check("Skipping cpp_tests.rs" in out, "C++ tests skipped when there is no C++ code")


def main():
    if not PLAYDSP.exists():
        raise SystemExit(f"{PLAYDSP} not found - run `cargo build --release` first")
    keep = "--keep" in sys.argv
    root = Path(tempfile.mkdtemp(prefix="playdsp_e2e_"))
    print(f"playdsp e2e tests in {root}")
    try:
        audio = test_formats_and_null(root)
        test_incremental_rebuild(audio)
        test_dsp_test_harness(audio)
        test_runtime_selftests(audio)
        test_new_does_not_overwrite(root, audio)
        test_code_import(root)
        test_tail_latency_and_sample_rate(root)
        test_rust_only_and_precision(root)
    finally:
        if keep:
            print(f"\nkept {root}")
        else:
            shutil.rmtree(root, ignore_errors=True)

    if FAILURES:
        print(f"\n{len(FAILURES)} check(s) failed:")
        for f in FAILURES:
            print(f"  - {f}")
        sys.exit(1)
    print("\nall end-to-end checks passed")


if __name__ == "__main__":
    main()
