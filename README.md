# playdsp

[![CI](https://github.com/baintonaj/playdsp/actions/workflows/ci.yml/badge.svg)](https://github.com/baintonaj/playdsp/actions/workflows/ci.yml)

## Introduction

After nearly 7 years of audio programming in C++/JUCE/Xcode/Pro Tools, I became frustrated with the design process of Audio Digital Signal Processing (DSP) algorithms. I would write new code, and by the time the plug-in would compile, copy, and load into Pro Tools, I would lose immediacy with what I had written. So I made this Command-Line Audio Signal Processing Framework.

The framework has a JUCE-like structure for audio DSP backend processing in C++ and Rust. Feed it C++ and/or Rust code, and one or more WAV files, and it will write a new 32-bit float WAV per input WAV file per programming language, processed by the code you give it.


## Overview

High-performance tool that compiles and executes Rust and/or C++ DSP code against audio files in parallel. Write your audio processing algorithms in either language, and playdsp handles compilation and execution automatically.

## Features

- **Dual-language support**: Write DSP code in Rust or C++ (or both)
- **On-the-fly compilation**: Automatically compiles your code locally on each run
- **Native folder structures**: Drop in entire Rust/C++ libraries with their native project structure
- **Automatic dependency management**: Rust external crates are auto-detected from all files recursively
- **Multi-file projects**: Full support for complex Rust modules and C++20 with nested subdirectories
- **Persistent state objects**: Create classes/structs that maintain state across buffer calls
- **Parallel processing**: Processes multiple audio files concurrently using Rayon
- **DSP unit testing**: `playdsp test` compiles and runs standard Rust `#[test]` functions against your DSP code without needing audio files; Rust and C++ tests run in one `cargo test` invocation (single-threaded, because DSP state is global)
- **Portable**: No installation of source files required - main binary is self-contained
- **BWF metadata passthrough**: Optional `--meta` flag preserves the `bext` chunk (description, originator, UMID, loudness metadata, timecode) from input files in the output — essential for Pro Tools and other pro audio applications
- **Format support**: 8/16/24/32-bit integer PCM and 32/64-bit float WAV files (`.wav` or `.WAV`), decoded straight to f64 with no intermediate f32 step
- **Fixed buffer size**: 1024 samples per buffer for all sample rates
- **Automatic reverb tail capture**: Every run pads audio with 1s of silence before and 12s after; output ends after the last 1024-sample window above -144 dBFS, so reverb and delay tails (including late echoes after a silent gap) are fully captured
- **Runtime helpers**: `playdsp_sample_rate()` gives your DSP the file's sample rate; `playdsp_set_latency(n)` reports look-ahead latency so the output is shifted back into alignment (like DAW plugin delay compensation)
- **Incremental builds**: generated runtime files are only rewritten when their content changes, so re-running with unchanged code skips compilation
- **Cross-platform paths**: PathBuf-based path construction for Windows, macOS, and Linux
- **Clean terminal output**: Spinner during runtime compilation (cargo output suppressed, shown only on error); per-file results printed thread-safely above a progress bar during audio processing
- **Auto SIMD**: f64→f32 conversion uses AVX intrinsics with scalar fallback on supported hardware

## Installation

```bash
cargo build --release
```

Binary will be at: `target/release/playdsp`

See [INSTALL.md](INSTALL.md) for system-wide installation options.

## Quick Start

### 1. Create Project Structure

```bash
playdsp new
```

This creates:
```
audio/
├── source/                        # Input WAV files
├── processing/                    # DSP code root
│   ├── rust/                      # Rust DSP code (with subdirectories)
│   │   └── rust_process_audio.rs  # Rust entry point
│   ├── cpp/                       # C++ DSP code (with subdirectories)
│   │   └── cpp_process_audio.cpp  # C++ entry point
│   └── tests/                     # DSP unit tests (run via playdsp test)
│       ├── rust_tests.rs          # Rust DSP tests
│       └── cpp_tests.rs           # C++ DSP tests
└── result/                        # Processed audio output
```

### 2. Write Your DSP Code

**For Rust** - Edit `audio/processing/rust/rust_process_audio.rs`:
```rust
use std::sync::{LazyLock, Mutex};

struct State {
    // Add per-channel DSP state here.
    // Example: prev_sample: Vec<f64>,
}

impl State {
    fn process(&mut self, input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
        let gain_db = -12.0;
        let gain_linear = 10.0_f64.powf(gain_db / 20.0);

        for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
            for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
                *out_sample = in_sample * gain_linear;
            }
        }
    }
}

static STATE: LazyLock<Mutex<State>> =
    LazyLock::new(|| Mutex::new(State {}));

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    STATE.lock().unwrap().process(input, output);
}
```

**For C++** - Edit `audio/processing/cpp/cpp_process_audio.cpp`:
```cpp
#include <cstddef>
#include <cmath>
#include <mutex>
#include <vector>

struct State {
    // Add per-channel DSP state here.
    // Example: std::vector<double> prev_sample;

    void process(std::vector<std::vector<double>>& input_vector,
                 std::size_t num_channels, std::size_t num_samples,
                 std::vector<std::vector<double>>& output_vector) {
        double gain_db     = -12.0;
        double gain_linear = std::pow(10.0, gain_db / 20.0);

        for (std::size_t channel = 0; channel < num_channels; channel++) {
            for (std::size_t sample = 0; sample < num_samples; sample++) {
                output_vector[channel][sample] =
                    input_vector[channel][sample] * gain_linear;
            }
        }
    }
};

extern "C" void cpp_process(const double* input, size_t num_channels,
                             size_t num_samples, double* output) {
    static std::mutex                        state_mutex;
    static State                             state;
    static std::vector<std::vector<double>>  input_vector;
    static std::vector<std::vector<double>>  output_vector;
    std::scoped_lock lock(state_mutex);

    if (input_vector.size() != num_channels ||
        (!input_vector.empty() && input_vector[0].size() != num_samples)) {
        input_vector.assign(num_channels,  std::vector<double>(num_samples, 0.0));
        output_vector.assign(num_channels, std::vector<double>(num_samples, 0.0));
    }

    std::size_t k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++)
        for (std::size_t channel = 0; channel < num_channels; channel++)
            input_vector[channel][sample] = input[k++];

    state.process(input_vector, num_channels, num_samples, output_vector);

    k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++)
        for (std::size_t channel = 0; channel < num_channels; channel++)
            output[k++] = output_vector[channel][sample];
}
```

**For Multi-file C++ Projects**: Place all `.cpp`, `.h`, and `.hpp` files in `audio/processing/cpp/` with any folder structure:
```
audio/processing/cpp/
├── cpp_process_audio.cpp  # Entry point
├── my_dsp_library.h
├── my_dsp_library.cpp
└── filters/
    ├── biquad.h
    └── biquad.cpp
```

```cpp
// In cpp_process_audio.cpp
#include "my_dsp_library.h"
#include "filters/biquad.h"

extern "C" void cpp_process(const double* input, size_t num_channels,
                            size_t num_samples, double* output) {
    MyDSP::process(input, num_channels, num_samples, output);
}
```

All `.cpp` files in all subdirectories are automatically compiled and linked.

**For Multi-file Rust Projects**: Place all `.rs` files in `audio/processing/rust/` with native Rust module structure:
```
audio/processing/rust/
├── rust_process_audio.rs  # Entry point
├── my_dsp.rs              # Your module
└── filters/
    ├── mod.rs
    └── biquad.rs
```

```rust
// In rust_process_audio.rs
mod my_dsp;
mod filters;

use my_dsp::MyProcessor;
use filters::biquad::Biquad;

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    // Use your modules
    MyProcessor::process(input, output);
}
```

The entire `rust/` folder is copied to the runtime as a module.

**For Rust Projects with External Crates**: playdsp automatically detects and includes external dependencies from all `.rs` files!

Two options:
1. **Auto-detection** (easiest): Just use the crate in your code
```rust
use rand::Rng;  // playdsp will auto-detect and add rand = "*" to Cargo.toml

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let mut rng = rand::thread_rng();
    // Your DSP code...
}
```

2. **Explicit versions** (recommended): Create `audio/processing/rust/dependencies.toml`
```toml
[dependencies]
rand = "0.8"
rustfft = { version = "6.0", features = ["avx"] }
```

See [DEPENDENCIES.md](DEPENDENCIES.md) for full documentation on dependency management.

### 3. Test Your DSP Code

```bash
playdsp test          # run all tests (Rust + C++)
playdsp test --rust   # run only Rust tests
playdsp test --cpp    # run only C++ tests
```

The starter files `rust_tests.rs` and `cpp_tests.rs` are ready to run immediately. They verify the default −12 dB gain behaviour, check that silence in produces silence out, and assert that buffer dimensions are preserved.

**Output example:**
```
DSP code detected - recompiling for test...
   Compiling playdsp_runtime v0.4.0

running 6 tests
test user_code::cpp_tests::test_cpp_buffer_dimensions_preserved ... ok
test user_code::cpp_tests::test_cpp_gain_minus_12db ... ok
test user_code::cpp_tests::test_cpp_silence_in_silence_out ... ok
test user_code::rust_tests::test_buffer_dimensions_preserved ... ok
test user_code::rust_tests::test_gain_minus_12db ... ok
test user_code::rust_tests::test_silence_in_silence_out ... ok

test result: ok. 6 passed; 0 failed; 0 ignored
```

Tests call `rust_process()` / `crate::cpp_process_audio_wrapper()` directly with synthetic buffers — no audio files needed. Add your own `#[test]` functions to either file, or create new `.rs` files in `tests/`. Files prefixed with `cpp_` are treated as C++ tests; all others are Rust tests.

**Note on state**: the Rust `State` and C++ statics are global singletons. For stateless DSP (gain, EQ) tests are fully independent. For stateful DSP (filters, delays) call the process function a few times first to flush transient state, or reset `State` fields manually between tests.

### 4. Add Audio Files

Place `.wav` files in `audio/source/`

### 4. Run Processing

```bash
cd /path/to/your/project

playdsp
playdsp --rust
playdsp --cpp
```

**What happens:**
- On first run, playdsp automatically compiles the runtime binary with your DSP code
- Processes all `.wav` files from `audio/source/`
- Outputs to `audio/result/` with timestamps: `{filename}_processed_{timestamp}_{rs|cpp}.wav`
- Subsequent runs reuse the compiled runtime (unless you modify your DSP code)

## Usage

```bash
playdsp [OPTIONS] [SUBCOMMAND]
```

### Options

- `-r`, `--rust`        Process with Rust code only
- `-c`, `--cpp`         Process with C++ code only
- `-m`, `--meta`        Preserve BWF metadata (`bext` chunk) from input WAV files in output
- `-d`, `--code <DIR>`  Use code from specified directory (copies to `audio/processing/rust/` or `audio/processing/cpp/`)
- `-a`, `--audio <DIR>` Use audio from specified directory (copies to `audio/source/`)
- `-h`, `--help`        Print help
- `-V`, `--version`     Print version

### Subcommands

- `new [--dir <DIR>]`            Create folder structure for DSP processing
- `test [-r|--rust] [-c|--cpp]`  Compile and run DSP tests from `audio/processing/tests/`

### Examples

Create folder structure:
```bash
playdsp new
playdsp new --dir /path/to/project
```

Run DSP tests:
```bash
playdsp test             # run all tests (Rust + C++)
playdsp test --rust      # run only Rust tests
playdsp test --cpp       # run only C++ tests
```

Process audio files:
```bash
playdsp
playdsp --rust
playdsp --cpp
playdsp --meta           # preserve BWF bext chunk in output files
playdsp --rust --meta
```

Import code and audio:
```bash
playdsp --code ../my-dsp-code --audio ../my-audio-files
```

## How It Works

1. **Setup**: When you run playdsp, it automatically checks for a compiled runtime binary
2. **Auto-Compilation** (if runtime doesn't exist or code changes detected):
   - Creates local runtime project at `../audio/.playdsp_runtime/`
   - Generates runtime binary from embedded templates
   - Copies entire `rust/` folder to runtime's `src/user_code/` module (if present)
   - Recursively scans all `.rs` files for external crate dependencies
   - Recursively compiles all `.cpp` files from `cpp/` folder with C++20 (if present)
   - Supports nested subdirectories for both languages
   - Shows indicatif spinner during cargo build; cargo output is suppressed and shown only on error; elapsed compile time printed on success
3. **Audio Processing**:
   - All input formats (8/16/24/32-bit PCM, 32/64-bit float) converted directly to f64
   - Audio padded with 1s of silence before and 12s after; full padded signal passes through user DSP
   - Non-finite output samples (NaN/Inf) from either language are replaced with 0.0, with one warning per file
   - Output shifted by any latency the DSP reported via `playdsp_set_latency()`
   - Output ends after the last 1024-sample window at or above -144 dBFS after source end (reverb tail capture)
   - Per-file results printed above the progress bar via `pb.println()` (thread-safe); progress bar tracks total file count with elapsed time
4. **Output**: Processed files saved as `{filename}_processed_{timestamp}_{rs|cpp}.wav` (32-bit float)

**Recompiling After Code Changes:**
- Runtime automatically recompiles when it detects code in `rust/` or `cpp/` folders; if nothing changed, cargo finishes almost instantly
- Or delete `../audio/.playdsp_runtime/` to force full recompilation
- Or use `--code ./processing` to explicitly trigger recompilation

## DSP Function Requirements

### Rust

```rust
pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) { }
```

- `input[channel][sample]` - Input audio (f64, normalized -1.0 to 1.0)
- `output[channel][sample]` - Output audio (f64, normalized -1.0 to 1.0)
- Buffer size: Fixed at 1024 samples

### C++

```cpp
extern "C" void cpp_process(const double* input, size_t num_channels,
                            size_t num_samples, double* output)
```

- `input` - Interleaved audio: [ch0_s0, ch1_s0, ch0_s1, ch1_s1, ...]
- `num_channels` - Number of audio channels
- `num_samples` - Fixed at 1024 samples per buffer
- `output` - Output buffer (same interleaved layout)

### Buffer Size

Fixed at **1024 samples per buffer** for all sample rates.

## Persistent State

`rust_process()` and `cpp_process()` are called once per 1024-sample buffer. Local variables are destroyed at the end of each call — so filter states, delay-line heads, envelope followers, and any data that must persist between buffers must live **outside** the function.

The starter files generated by `playdsp new` already scaffold this pattern. The entry-point functions are lock-and-delegate wrappers; all DSP logic lives inside `State::process()`.

### Rust — `LazyLock<Mutex<State>>`

Add fields to `State` for any data that must persist across buffer calls, then use them inside `State::process()`. Size per-channel `Vec`s lazily — the channel count is only known at call time:

```rust
struct State {
    prev_sample: Vec<f64>,
}

impl State {
    fn process(&mut self, input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
        if self.prev_sample.len() < input.len() {
            self.prev_sample.resize(input.len(), 0.0);
        }
        // use self.prev_sample in your DSP loop
    }
}
```

### C++ — `State::process()` + `std::scoped_lock`

Add fields to `State` and use them inside `State::process()`. Size per-channel vectors lazily — the channel count is only known at call time:

```cpp
struct State {
    std::vector<double> prev_sample;

    void process(std::vector<std::vector<double>>& input_vector,
                 std::size_t num_channels, std::size_t num_samples,
                 std::vector<std::vector<double>>& output_vector) {
        if (prev_sample.size() < num_channels)
            prev_sample.resize(num_channels, 0.0);
        // use prev_sample in your DSP loop
    }
};
```

The static `input_vector` and `output_vector` in `cpp_process()` are reused every call, avoiding repeated heap allocation. All four statics (`state_mutex`, `state`, `input_vector`, `output_vector`) are protected by `std::scoped_lock`.

## Runtime Helpers

The runtime provides two functions your DSP can call, in either language:

| Rust | C++ | Purpose |
|---|---|---|
| `crate::playdsp_sample_rate() -> f64` | `extern "C" double playdsp_sample_rate();` | Sample rate of the file being processed (48000 under `playdsp test`) |
| `crate::playdsp_set_latency(samples: usize)` | `extern "C" void playdsp_set_latency(size_t samples);` | Report processing latency; the output is shifted back by this many samples |

The C++ starter file already declares both functions.

## Technical Details

- **C++ Standard**: C++20
- **Optimization**: Platform-conditional — `-O3` on GCC/Clang; `/O2` + `/EHsc` on MSVC. Linux also adds `-fPIC`.
- **SIMD**: AVX intrinsics for f64→f32 sample conversion in a `#[target_feature(enable = "avx")]` function, selected at runtime; scalar fallback on non-AVX and non-x86 hardware
- **Release profile**: LTO + single codegen unit for the runtime binary
- **MSRV**: Rust 1.85 (required for edition 2024)
- **Input Audio Formats**: 8/16/24/32-bit integer PCM and 32-bit float via bwavfile; 64-bit float read directly from the data chunk
- **Processing Format**: All audio automatically converted to 64-bit float (-1.0 to 1.0)
- **Output Format**: 32-bit float WAV (IEEE 754)
- **BWF Metadata**: `bext` chunk (originator, description, UMID, loudness tags, timecode) read on every run; written to output only when `--meta` is passed
- **Parallelism**: Rayon for concurrent file processing
- **Buffer Size**: Fixed at 1024 samples per buffer

## Error Handling

The tool provides clear error messages for:
- Missing or invalid file paths
- Compilation errors in user code
- Audio file read/write failures
- Incorrect DSP function signatures
- Unsupported audio formats (compressed or non-PCM WAV)

## Testing playdsp Itself

```bash
make test    # CLI unit tests (cargo test)
make e2e     # release build + end-to-end suite (python3 ci/e2e_test.py)
```

`ci/e2e_test.py` uses only the Python standard library. It creates throwaway projects with `playdsp new`, writes WAV files in every supported format, runs the real binary, and checks the output sample by sample. It covers gain accuracy, Rust vs C++ null tests, tail capture, latency compensation, Rust-only projects, `--code` import, `playdsp new` safety, no-op rebuilds, and the runtime self-tests (AVX vs scalar conversion, tail detection).

GitHub Actions (`.github/workflows/ci.yml`) runs the same suite on Linux x86_64, Windows x86_64 (MSVC), Linux arm64, and macOS arm64, plus an MSRV job on Rust 1.85. On the x86_64 runners `PLAYDSP_REQUIRE_AVX=1` makes the AVX self-test fail instead of skip, so CI proves the SIMD path runs.

## Requirements

- Rust toolchain 1.85+ (for building playdsp; required for edition 2024)
- C++ compiler (for C++ DSP code support)
- Cargo (included with Rust)

## Version History

### Unreleased

**Audio fixes**
- 64-bit float WAV input no longer panics (bwavfile has no f64 reader, so the data chunk is read directly).
- 32-bit integer input keeps full precision: samples are decoded via i32 to f64 instead of through f32.
- 8-bit WAV input is now supported.
- Tail capture keeps late echoes: output ends after the *last* window above -144 dBFS rather than the first quiet one.
- NaN/Inf protection now covers Rust as well as C++, with one summary warning per file instead of one per buffer.
- Unknown runtime mode and unsupported formats return clear errors instead of silent zeros or panics.
- Runtime no longer holds whole-file block arrays; processing uses two reusable 1024-sample buffers.

**New runtime helpers**
- `playdsp_sample_rate()` and `playdsp_set_latency()` for Rust and C++ (see Runtime Helpers).

**Tooling fixes**
- Runtime builds and runs with no C++ code (previously failed to link with an undefined `cpp_process` symbol).
- `--code` signature check is whitespace-insensitive (it previously rejected normally formatted code, including the starter files).
- `--code` no longer compiles the runtime twice.
- `playdsp new` no longer overwrites existing DSP or test files.
- `.WAV` (uppercase) files are processed; `--audio` only replaces WAV files in `source/`.
- Dependency detection handles `use x as y`, `pub use`, `use ::x`, `extern crate`, and sibling module files; test files are scanned during `playdsp test`.
- Unchanged code no longer triggers a rebuild; C++ file additions/removals are still detected.
- `playdsp test` runs single-threaded and skips `cpp_` tests when there is no C++ code.
- `-r -c` together means both languages; runtime warnings print above the progress bar instead of over it.
- Unit tests added for signature validation, dependency detection, and template patching (`cargo test`).
- Windows: template line endings are normalised before patching. With `core.autocrlf` the CRLF template never matched the patch markers, so user Rust code was silently ignored. Missing markers are now an error.

**CI**
- GitHub Actions on Linux x86_64, Windows x86_64 (MSVC), Linux arm64, macOS arm64, and MSRV 1.85.
- Portable end-to-end suite (`ci/e2e_test.py`, `make e2e`) and CI-only runtime self-tests (`--cfg playdsp_selftest`).

---

### v0.4.0 (March 2026)

**DSP unit testing**
- **`playdsp test` subcommand**: recompiles the runtime in test mode and runs standard Rust `#[test]` functions. Full `cargo test` output is shown — panics, assertion failures, and line numbers are all visible.
- **`--rust` / `--cpp` flags on `test`**: `playdsp test --rust` runs only Rust tests; `playdsp test --cpp` runs only C++ tests; default runs both.
- **Starter test files**: `playdsp new` now writes `processing/tests/rust_tests.rs` and `processing/tests/cpp_tests.rs` with three ready-to-run tests each — verifying the default −12 dB gain, silence-in/silence-out, and buffer dimension preservation.
- **C++ tests via safe wrapper**: C++ test files call `crate::cpp_process_audio_wrapper()` directly, the same safe interleave/deinterleave wrapper used during audio file processing — no raw pointer arithmetic in test code.
- **File naming convention**: files in `tests/` prefixed with `cpp_` are treated as C++ tests; all others are Rust tests. This drives `--rust`/`--cpp` filtering.
- **Clean separation**: test files are injected only during `playdsp test`, not during normal audio processing builds.

**Project structure**
- `playdsp new` creates `audio/processing/tests/` alongside `rust/` and `cpp/`.

---

### v0.3.1 (March 2026)

**Audio**
- **BWF metadata passthrough** (`--meta` / `-m`): when passed, the `bext` chunk is read from each input WAV file and written to the corresponding output file unchanged. Preserves originator, description, UMID, timecode reference, and EBU R128 loudness tags — essential for round-tripping files through Pro Tools and other BWF-aware DAWs. Without the flag (default) no metadata is copied, matching previous behaviour.

---

### v0.3.0 (February 2026)

**Audio processing**
- **Reverb tail capture (always-on)**: every run now pre-pads 1 second and post-pads 12 seconds of silence around the source audio, processes the full padded signal, then trims the output at the first 1024-sample window that falls below −144 dBFS (hard-capped at 12 seconds post-source). DSP with no reverb tail (gain, EQ, clipping) is unaffected — the RMS drops below threshold immediately and output length is unchanged.
- **AVX SIMD f64→f32 conversion**: uses `_mm256_cvtpd_ps` with a runtime `is_x86_feature_detected!` guard and a scalar fallback for non-AVX hardware.
- **C++ NaN/Inf validation**: output buffer validated after every FFI call; non-finite values are clamped to 0.0 with a warning.
- **WAV write error handling**: `unwrap()` calls in `write_wav` replaced with proper `?` propagation.

**Compilation**
- **Release profile optimised**: runtime `Cargo.toml` now sets `lto = true` and `codegen-units = 1` for smaller, faster binaries.
- **Cross-platform C++ flags**: MSVC gets `/O2 /std:c++20 /EHsc`; GCC/Clang get `-O3 -std=c++20`; Linux additionally gets `-fPIC`.
- **Windows binary path**: runtime resolved with `std::env::consts::EXE_SUFFIX` so `playdsp_runtime.exe` is found correctly on Windows.
- **MSRV pinned**: `rust-version = "1.85"` added to `Cargo.toml` (required for edition 2024).

**Terminal output**
- **Animated spinner** with elapsed time during runtime compilation (`indicatif`).
- **Progress bar** (cyan/blue) tracking all `(audio file × program)` pairs during processing, with elapsed time.
- **Clean output**: cargo build stdout/stderr suppressed during compilation — output is shown only if compilation fails. Per-file results printed above the progress bar via `pb.println()` (thread-safe, no interleaving).

**Parallelism**
- Processing refactored to a single flattened `par_iter` over all `(audio_file, program_path)` pairs instead of nested parallel iterators.
- Explicit thread pool initialisation via `rayon::ThreadPoolBuilder` with `available_parallelism()`.

**Developer experience**
- `CLAUDE.md` added with full architecture documentation for AI-assisted development.
- `Makefile` added with targets: `all`, `build`, `release`, `install`, `reinstall` (clean + rebuild + install in one step), `install-cargo`, `uninstall`, `clean`, `help`.

---

### v0.2.0

- Initial public release.
- Dual-language support: Rust and C++ DSP code compiled and executed against WAV files.
- `new` subcommand to scaffold the `audio/` folder structure.
- Auto-detection of external Rust crate dependencies from `use` statements across all `.rs` files.
- `dependencies.toml` for explicit dependency versions and feature flags.
- Multi-file Rust module support: entire `rust/` folder copied into the runtime as a `user_code` module.
- Multi-file C++ support: all `.cpp`/`.h`/`.hpp` files in `cpp/` and its subdirectories compiled and linked.
- Parallel audio processing with Rayon across all input files.
- `-d`/`--code` and `-a`/`--audio` flags to import code and audio from external directories.
- 16-bit, 24-bit, and 32-bit integer PCM and 32-bit float WAV input; 32-bit float WAV output.

## License

MIT License
