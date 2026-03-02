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

There are no tests or linting configured in this project.

## Architecture

PlayDSP is a CLI tool that compiles and executes user-written Rust and/or C++ DSP code against WAV audio files in parallel. It has a **two-binary architecture**: the main CLI tool orchestrates everything, and a **runtime binary** is dynamically generated and compiled from embedded templates.

### Execution Flow

1. **CLI parsing** (`src/main.rs`) — clap-based argument parsing with `new` subcommand and `-r`/`-c`/`-d`/`-a`/`-m` flags. Initialises a Rayon thread pool via `ThreadPoolBuilder` at startup.
2. **File management** (`src/file_processing/`) — copies user code and audio files to standard locations under `../audio/`
3. **Runtime compilation** (`src/program_recompile/run_recompile.rs`) — the core orchestration:
   - Creates `.playdsp_runtime/` project from embedded templates (`templates/`)
   - Scans user Rust code for `use` statements to auto-detect crate dependencies
   - Merges auto-detected deps with explicit `dependencies.toml` entries
   - Copies user Rust code as a `user_code` module and patches `main.rs` to delegate to it
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
│   └── cpp/                # User C++ code (entry: cpp_process_audio.cpp)
└── result/                 # Output WAV files (timestamped)
```

All paths are defined as `static LazyLock<PathBuf>` in `src/constants/constants.rs` using `PathBuf::from("..").join(...)` for OS-portable construction. The runtime binary name is resolved with `std::env::consts::EXE_SUFFIX` for Windows compatibility.

### Template System

Three templates in `templates/` are embedded at compile time via `include_str!` in `run_recompile.rs`:
- `Cargo.toml.template` — runtime manifest; user dependencies are injected after `[dependencies]`. Includes `[profile.release]` with `lto = true` and `codegen-units = 1`.
- `main.rs.template` — runtime entry point with WAV I/O (bwavfile), Rust/C++ dispatch, buffer processing (1024-sample chunks), AVX SIMD f64→f32 conversion, post-FFI NaN/Inf validation, reverb tail padding logic, and optional BWF `bext` metadata passthrough. Contains marker comments that get patched to wire in user code.
- `build.rs.template` — recursively finds and compiles C++ files with `cc`. Uses platform-conditional flags: `-O3`/`-std=c++20` on GCC/Clang, `/O2`/`/std:c++20`/`/EHsc` on MSVC, `-fPIC` added on Linux.

### Dependency Detection

`run_recompile.rs` implements a two-tier dependency system:
1. **Explicit**: `dependencies.toml` in the rust folder (parsed manually, supports feature flags)
2. **Auto-detected**: scans all `.rs` files for `use` statements, excludes `std`/`core`/`alloc` and locally-declared modules, assigns version `"*"`

### Audio Processing

- Buffer size: 1024 samples (fixed)
- Input: 16/24/32-bit integer PCM or 32/64-bit float WAV (no 8-bit)
- Output: 32-bit float WAV
- Audio normalised to f64 `[-1.0, 1.0]` for processing
- C++ FFI uses `extern "C"` with flattened interleaved buffers; output validated for NaN/Inf after each call
- f64→f32 conversion uses AVX intrinsics (`_mm256_cvtpd_ps`) with `is_x86_feature_detected!` runtime guard and scalar fallback
- BWF metadata (`bext` chunk) is always read from input via `WaveReader::broadcast_extension()`; written to output via `WaveWriter::write_broadcast_metadata()` only when the `--meta` / `-m` flag is passed. Without the flag the `bext` chunk is discarded (default behaviour). `write_broadcast_metadata` must be called before `audio_frame_writer()` — the template enforces this ordering.

### Reverb Tail Capture (always-on)

On every run the runtime automatically:
1. Prepends `sample_rate` samples (1 second) of zeros per channel
2. Appends `sample_rate * 12` samples (12 seconds) of zeros per channel
3. Processes the entire padded signal through user DSP sequentially
4. Walks the post-source region in 1024-sample windows computing per-window RMS
5. Trims output at the first window below -144 dBFS (`6.31e-8`), hard-capped at 12 seconds post-source
6. The pre-pad region is discarded from the output

For DSP with no tail (gain, EQ, clipper) the RMS drops below threshold immediately at source end, so the output length is unchanged.

### Required User Function Signatures

**Rust**: `pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>)` in `rust_process_audio.rs`

**C++**: `extern "C" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output)` in `cpp_process_audio.cpp`

Signature validation is whitespace-normalised — extra spaces and newlines in the user's file are tolerated.

### Persistent State

Both entry-point functions are called once per 1024-sample buffer. Local variables are destroyed at the end of each call, so filter states, delay-line read/write heads, envelope followers, and any other cross-buffer data must live outside the function.

The starter files written by `playdsp new` (`create_folders_and_copy_files.rs`) scaffold this pattern by default — the entry-point functions are lock-and-delegate wrappers; all DSP logic lives inside `State::process()`:

- **Rust**: `static STATE: LazyLock<Mutex<State>>` — initialised once on the first buffer call. `rust_process()` calls `STATE.lock().unwrap().process(input, output)` and returns. Add fields to `State` and implement them in `State::process()`. Per-channel `Vec`s are grown lazily because channel count is only known at call time.
- **C++**: Four statics inside `cpp_process()` — `state_mutex`, `state`, `input_vector`, `output_vector` — all protected by `std::scoped_lock` (C++17, full mutual exclusion; not just init-safe). `input_vector`/`output_vector` are reused every call, eliminating per-buffer heap allocation; they are resized lazily when dimensions change. `State::process()` receives the pre-deinterleaved 2D vectors by reference and does only DSP — no raw pointer arithmetic inside DSP logic. Add fields to `State` and implement them in `State::process()`.

### MSRV

Requires **rustc 1.85+** (`edition = "2024"`, `rust-version = "1.85"` in `Cargo.toml`). The runtime template uses edition 2021 and has no special MSRV constraint beyond what `bwavfile` and `cc` require.
