# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build --release          # Build the CLI tool
cargo install --path .         # Install to ~/.cargo/bin/
cargo run -- new               # Create project folder structure in current dir
cargo run -- new --dir <path>  # Create project folder structure at <path>
cargo run                      # Run with both Rust and C++ processing
cargo run -- --rust            # Run with Rust only
cargo run -- --cpp             # Run with C++ only
cargo run -- -d <dir>          # Import code from external directory
cargo run -- -a <dir>          # Import audio from external directory
cargo run -- --meta            # Preserve BWF bext chunk in output files
cargo run -- --rust --meta     # Rust only, with BWF metadata passthrough
cargo run -- test              # Compile and run all DSP tests (Rust + C++ in parallel)
cargo run -- test --rust       # Run only Rust DSP tests
cargo run -- test --cpp        # Run only C++ DSP tests
```

Or via the Makefile:

```bash
make                           # Release build (default)
make build                     # Debug build
make release                   # Release build
sudo make install              # Install to /usr/local/bin (Unix default)
sudo make reinstall            # Clean + rebuild + install in one step
make install-cargo             # Install via cargo install (cross-platform)
sudo make uninstall            # Remove installed binary
make help                      # Show all targets and current DESTDIR
```

```bash
make test                      # CLI unit tests (cargo test)
make e2e                       # release build + python3 ci/e2e_test.py (end-to-end suite)
```

There is no linting configured. `cargo test` runs the CLI's own unit tests (signature validation, dependency detection, template patching). DSP unit tests run via `playdsp test` (see DSP Test Framework below).

### Testing & CI

- **Local testing is Apple Silicon only** (`make test`, `make e2e`). Don't cross-compile or use Rosetta for x86_64 locally; CI covers it.
- `ci/e2e_test.py` (stdlib-only Python) builds throwaway projects in a temp dir and verifies rendered audio sample by sample: all input formats, Rust/C++ null, tail capture after a silent gap, latency compensation, Rust-only projects, `-d` import, `new` safety, no-op rebuild (runtime binary mtime unchanged), and runtime self-tests.
- Runtime self-tests live at the bottom of `templates/main.rs.template` under `#[cfg(all(test, playdsp_selftest))]`; the e2e script runs them with `RUSTFLAGS="--cfg playdsp_selftest" cargo test runtime_selftests` inside `.playdsp_runtime`. They never appear in a user's `playdsp test`. `PLAYDSP_REQUIRE_AVX=1` makes the AVX test fail instead of skip.
- `.github/workflows/ci.yml`: matrix of Linux x86_64 (GCC), Windows x86_64 (MSVC), Linux arm64 (`ubuntu-24.04-arm`), macOS arm64, plus an MSRV 1.85 job. Each runs `cargo build --release --locked`, `cargo test --locked`, `python ci/e2e_test.py`.
- Templates must stay LF (`.gitattributes`); `patch_main_rs` also normalises CRLF, because a CRLF template would never match the patch markers.

## Architecture

PlayDSP is a CLI tool that compiles and executes user-written Rust and/or C++ DSP code against WAV audio files in parallel. It has a **two-binary architecture**: the main CLI tool orchestrates everything, and a **runtime binary** is dynamically generated and compiled from embedded templates.

### Execution Flow

1. **CLI parsing** (`src/main.rs`) — clap-based argument parsing with `new`/`test` subcommands and `-r`/`-c`/`-d`/`-a`/`-m` flags (`-r` and `-c` are `SetTrue`; neither or both means both languages). The runtime is compiled at most once per run. Initialises a Rayon thread pool via `ThreadPoolBuilder` at startup.
2. **File management** (`src/file_processing/`) — copies user code and audio files to standard locations under `../audio/`
3. **Runtime compilation** (`src/program_recompile/run_recompile.rs`) — the core orchestration:
   - Creates `.playdsp_runtime/` project from embedded templates (`templates/`)
   - Scans user Rust code for `use` statements to auto-detect crate dependencies
   - Merges auto-detected deps with explicit `dependencies.toml` entries
   - Syncs user Rust code into a `user_code` module (`sync_dir`: copies changed files, deletes stale ones) and patches `main.rs` in memory to delegate to it
   - All generated files go through `write_if_changed`, so unchanged sources leave mtimes alone and `cargo build` is a near no-op
   - Compiles C++ via the `cc` crate (C++20, `-O3` on GCC/Clang; `/O2`+`/EHsc` on MSVC)
   - Runs `cargo build --release` with stdout suppressed and stderr piped; `indicatif` spinner animates during compilation; stderr is surfaced only on failure; elapsed time printed on success
4. **Parallel processing** (`src/signal_processing/`) — uses Rayon to invoke the runtime binary concurrently across all `(audio_file, program_path)` pairs via a single flattened `par_iter`. Per-file results are printed above the bar via `pb.println()` (thread-safe); progress bar tracks total pair count with elapsed time. When `--meta` is set, `--meta` is appended to each runtime invocation.

### Key Directory Layout (runtime, relative to execution dir)

```
../audio/
├── .playdsp_runtime/       # Auto-generated runtime project (compiled binary)
├── source/                 # Input WAV files
├── processing/
│   ├── rust/               # User Rust code (entry: rust_process_audio.rs)
│   ├── cpp/                # User C++ code (entry: cpp_process_audio.cpp)
│   └── tests/              # User DSP test files (*.rs, run via playdsp test)
└── result/                 # Output WAV files (timestamped)
```

All paths are defined as `static LazyLock<PathBuf>` in `src/constants/constants.rs` using `PathBuf::from("..").join(...)` for OS-portable construction. The runtime binary name is resolved with `std::env::consts::EXE_SUFFIX` for Windows compatibility.

### Template System

Three templates in `templates/` are embedded at compile time via `include_str!` in `run_recompile.rs`:
- `Cargo.toml.template` — runtime manifest; user dependencies are injected after `[dependencies]`. Includes `[profile.release]` with `lto = true` and `codegen-units = 1`.
- `main.rs.template` — runtime entry point with WAV I/O (bwavfile), Rust/C++ dispatch, buffer processing (1024-sample chunks via two reusable block buffers), AVX SIMD f64→f32 conversion, NaN/Inf sanitising for both languages, latency compensation, reverb tail padding logic, and optional BWF `bext` metadata passthrough. Exports the runtime helpers `playdsp_sample_rate()` and `playdsp_set_latency()` as `#[no_mangle] extern "C"`. Contains marker comments that get patched to wire in user code. The C++ FFI is gated on `#[cfg(playdsp_has_cpp)]`; without C++ sources a stub `cpp_process_audio_wrapper` panics and the `cpp` mode returns an error.
- `build.rs.template` — recursively finds and compiles C++ files with `cc`, emits `cargo:rustc-cfg=playdsp_has_cpp` when any exist, and watches the `cpp/` directory with `rerun-if-changed` so added/removed files trigger a rebuild. Uses platform-conditional flags: `-O3`/`-std=c++20` on GCC/Clang, `/O2`/`/std:c++20`/`/EHsc` on MSVC, `-fPIC` added on Linux.

### Dependency Detection

`run_recompile.rs` implements a two-tier dependency system:
1. **Explicit**: `dependencies.toml` in the rust folder (parsed manually, supports feature flags)
2. **Auto-detected**: scans all `.rs` files (plus `tests/` during `playdsp test`) for `use` / `pub use` / `extern crate` lines, takes the leading identifier (handles `as` aliases and `::x`), excludes `std`/`core`/`alloc`/`crate`/`self`/`super`, locally-declared modules and sibling file stems, assigns version `"*"`. Dependencies are written sorted so Cargo.toml is stable between runs.

### Audio Processing

- Buffer size: 1024 samples (fixed)
- Input: 8/16/24/32-bit integer PCM (read as i32, scaled by 2^-31) or 32-bit float (bwavfile), 64-bit float (data chunk read directly — bwavfile has no f64 reader). `.wav` matched case-insensitively
- Output: 32-bit float WAV
- Audio normalised to f64 `[-1.0, 1.0]` for processing
- C++ FFI uses `extern "C"` with flattened interleaved buffers
- Every block's output (Rust or C++) is checked for NaN/Inf; non-finite samples become 0.0 and one warning per file is printed
- Runtime helpers: `playdsp_sample_rate()` (48000 default, e.g. under tests) and `playdsp_set_latency(n)`; latency is clamped to the post-pad and the output window starts at `pre_pad + latency`
- f64→f32 conversion uses AVX intrinsics (`_mm256_cvtpd_ps`) inside a `#[target_feature(enable = "avx")]` function, selected by an `is_x86_feature_detected!` runtime check, with scalar fallback
- BWF metadata (`bext` chunk) is always read from input via `WaveReader::broadcast_extension()`; written to output via `WaveWriter::write_broadcast_metadata(&Bext)` only when the `--meta` / `-m` flag is passed. Without the flag the `bext` chunk is discarded (default behaviour). `write_broadcast_metadata` must be called before `audio_frame_writer()` — the template enforces this ordering.

### Reverb Tail Capture (always-on)

On every run the runtime automatically:
1. Prepends `sample_rate` samples (1 second) of zeros per channel
2. Appends `sample_rate * 12` samples (12 seconds) of zeros per channel
3. Processes the entire padded signal through user DSP sequentially
4. Walks the post-source region in 1024-sample windows (final window may be partial) computing per-window RMS across all channels
5. Ends output after the **last** window at or above -144 dBFS (`6.31e-8`), hard-capped at 12 seconds post-source — scanning every window keeps late echoes that follow a silent gap
6. The pre-pad region (plus any reported latency) is discarded from the output

For DSP with no tail (gain, EQ, clipper) the RMS drops below threshold immediately at source end, so the output length is unchanged.

### Required User Function Signatures

**Rust**: `pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>)` in `rust_process_audio.rs`

**C++**: `extern "C" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output)` in `cpp_process_audio.cpp`

Signature validation strips all whitespace from both the file and the reference signature before matching, so any spacing or line wrapping is tolerated (`contains_signature` in `process_and_copy_files.rs`).

### Persistent State

Both entry-point functions are called once per 1024-sample buffer. Local variables are destroyed at the end of each call, so filter states, delay-line read/write heads, envelope followers, and any other cross-buffer data must live outside the function.

The starter files written by `playdsp new` (`create_folders_and_copy_files.rs`; existing files are never overwritten) scaffold this pattern by default — the entry-point functions are lock-and-delegate wrappers; all DSP logic lives inside `State::process()`:

- **Rust**: `static STATE: LazyLock<Mutex<State>>` — initialised once on the first buffer call. `rust_process()` calls `STATE.lock().unwrap().process(input, output)` and returns. Add fields to `State` and implement them in `State::process()`. Per-channel `Vec`s are grown lazily because channel count is only known at call time.
- **C++**: Four statics inside `cpp_process()` — `state_mutex`, `state`, `input_vector`, `output_vector` — all protected by `std::scoped_lock` (C++17, full mutual exclusion; not just init-safe). `input_vector`/`output_vector` are reused every call, eliminating per-buffer heap allocation; they are resized lazily when dimensions change. `State::process()` receives the pre-deinterleaved 2D vectors by reference and does only DSP — no raw pointer arithmetic inside DSP logic. Add fields to `State` and implement them in `State::process()`.

### DSP Test Framework (`playdsp test`)

`playdsp test` recompiles the runtime in test mode and runs standard Rust `#[test]` functions the user writes in `audio/processing/tests/`. No new crates are required.

**How it works:**
- `run_tests(rust_only, cpp_only)` (`src/program_recompile/run_tests.rs`) calls `setup_runtime_project(.., include_tests = true)` and `inject_user_rust_code` (reused from `run_recompile.rs`), then `inject_test_files` (defined in `run_tests.rs`) copies test files and appends `#[cfg(test)] mod` declarations to `mod.rs`, then runs `cargo test -- --test-threads=1` with all output inherited (not suppressed)
- `cpp_` test files are skipped when `processing/cpp/` has no C++ sources (`has_cpp_files` in `get_program_files.rs`)
- Test files are NOT injected during normal audio processing — only during `playdsp test`
- Rust test files use `use super::rust_process_audio::rust_process;` to call Rust DSP; C++ test files call `crate::cpp_process_audio_wrapper()` (the safe wrapper defined in `main.rs.template`)
- `playdsp new` writes two starter files: `rust_tests.rs` (Rust) and `cpp_tests.rs` (C++) to `processing/tests/`

**File naming convention for `--rust`/`--cpp` filtering**: files prefixed with `cpp_` are treated as C++ tests; all others are Rust tests. `inject_test_files` uses this to determine which files to copy based on the mode flag.

**State note**: `STATE` (Rust) and the C++ statics are global singletons; stateless DSP tests are independent, stateful DSP tests may need warm-up calls or manual state resets between tests. Tests run single-threaded (`--test-threads=1`) so stateful tests don't interleave calls into shared state.

### MSRV

Requires **rustc 1.85+** (`edition = "2024"`, `rust-version = "1.85"` in `Cargo.toml`). The runtime template uses edition 2021 and has no special MSRV constraint beyond what `bwavfile` and `cc` require.
