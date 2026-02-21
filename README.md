# playdsp

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
- **Portable**: No installation of source files required - main binary is self-contained
- **Format support**: 16-bit, 24-bit, 32-bit integer PCM and 32-bit float WAV files
- **Fixed buffer size**: 1024 samples per buffer for all sample rates
- **Automatic reverb tail capture**: Every run pads audio with 1s of silence before and up to 12s after; output is trimmed at -144 dBFS so reverb/delay tails are always fully captured
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
│   └── cpp/                       # C++ DSP code (with subdirectories)
│       └── cpp_process_audio.cpp  # C++ entry point
└── result/                        # Processed audio output
```

### 2. Write Your DSP Code

**For Rust** - Edit `audio/processing/rust/rust_process_audio.rs`:
```rust
pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let gain_db = -12.0;
    let gain_linear = 10.0_f64.powf(gain_db / 20.0);

    for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
        for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
            *out_sample = in_sample * gain_linear;
        }
    }
}
```

**For C++** - Edit `audio/processing/cpp/cpp_process_audio.cpp`:
```cpp
#include <cstddef>
#include <cmath>
#include <vector>

extern "C" void cpp_process(const double* input, size_t num_channels,
                            size_t num_samples, double* output) {
    std::vector<std::vector<double>> input_vector(num_channels,
        std::vector<double>(num_samples, 0.0));
    std::vector<std::vector<double>> output_vector(num_channels,
        std::vector<double>(num_samples, 0.0));

    std::size_t k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++) {
        for (std::size_t channel = 0; channel < num_channels; channel++) {
            input_vector[channel][sample] = input[k];
            k++;
        }
    }

    double gain_db = -12.0;
    double gain_linear = std::pow(10.0, gain_db / 20.0);

    for (std::size_t channel = 0; channel < num_channels; channel++) {
        for (std::size_t sample = 0; sample < num_samples; sample++) {
            output_vector[channel][sample] = input_vector[channel][sample] * gain_linear;
        }
    }

    k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++) {
        for (std::size_t channel = 0; channel < num_channels; channel++) {
            output[k] = output_vector[channel][sample];
            k++;
        }
    }
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

### 3. Add Audio Files

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

- `-r`, `--rust`      Process with Rust code only
- `-c`, `--cpp`       Process with C++ code only
- `-d`, `--code <DIR>` Use code from specified directory (copies to `audio/processing/rust/` or `audio/processing/cpp/`)
- `-a`, `--audio <DIR>` Use audio from specified directory (copies to `audio/source/`)
- `-h`, `--help`      Print help
- `-V`, `--version`   Print version

### Subcommands

- `new [--dir <DIR>]` Create folder structure for DSP processing

### Examples

Create folder structure:
```bash
playdsp new
playdsp new --dir /path/to/project
```

Process audio files:
```bash
playdsp
playdsp --rust
playdsp --cpp
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
   - All input formats (16/24/32-bit PCM, 32/64-bit float) converted to f64
   - Audio padded with 1s of silence before and 12s after; full padded signal passes through user DSP
   - Output trimmed at the first 1024-sample window below -144 dBFS after source end (reverb tail capture)
   - Per-file results printed above the progress bar via `pb.println()` (thread-safe); progress bar tracks total file count with elapsed time
4. **Output**: Processed files saved as `{filename}_processed_{timestamp}_{rs|cpp}.wav` (32-bit float)

**Recompiling After Code Changes:**
- Runtime automatically recompiles when it detects code in `rust/` or `cpp/` folders
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

## Technical Details

- **C++ Standard**: C++20
- **Optimization**: Platform-conditional — `-O3` on GCC/Clang; `/O2` + `/EHsc` on MSVC. Linux also adds `-fPIC`.
- **SIMD**: AVX intrinsics for f64→f32 sample conversion; scalar fallback on non-AVX hardware
- **Release profile**: LTO + single codegen unit for the runtime binary
- **MSRV**: Rust 1.85 (required for edition 2024)
- **Input Audio Formats**: 16/24/32-bit integer PCM, 32-bit float WAV (bwavfile handles conversion)
- **Processing Format**: All audio automatically converted to 64-bit float (-1.0 to 1.0)
- **Output Format**: 32-bit float WAV (IEEE 754)
- **Parallelism**: Rayon for concurrent file processing
- **Buffer Size**: Fixed at 1024 samples per buffer
- **8-bit audio**: Not supported

## Error Handling

The tool provides clear error messages for:
- Missing or invalid file paths
- Compilation errors in user code
- Audio file read/write failures
- Incorrect DSP function signatures
- Unsupported audio formats (8-bit)

## Requirements

- Rust toolchain 1.85+ (for building playdsp; required for edition 2024)
- C++ compiler (for C++ DSP code support)
- Cargo (included with Rust)

## Version History

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
