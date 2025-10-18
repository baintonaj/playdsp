# playdsp

Command-Line Audio Signal Processing Framework

High-performance tool that compiles and executes Rust and/or C++ DSP code against audio files in parallel. Write your audio processing algorithms in either language, and playdsp handles compilation and execution automatically.

## Features

- **Dual-language support**: Write DSP code in Rust or C++ (or both)
- **On-the-fly compilation**: Automatically compiles your code locally on each run
- **Multi-file C++ projects**: Full support for C++20 with headers and multiple source files
- **Parallel processing**: Processes multiple audio files concurrently using Rayon
- **Portable**: No installation of source files required - main binary is self-contained
- **Format support**: 16-bit, 24-bit, 32-bit integer PCM and 32-bit float WAV files
- **Fixed buffer size**: 1024 samples per buffer for all sample rates

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
├── source/            # Input WAV files
├── processing/        # DSP code (rust_process_audio.rs, cpp_process_audio.cpp)
└── result/            # Processed audio output
```

### 2. Write Your DSP Code

**For Rust** - Edit `audio/processing/rust_process_audio.rs`:
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

**For C++** - Edit `audio/processing/cpp_process_audio.cpp`:
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

**For Multi-file C++ Projects**: Place all `.cpp`, `.h`, and `.hpp` files in `audio/processing/`:
```cpp
#include "my_dsp_library.h"
#include "filters.h"

extern "C" void cpp_process(const double* input, size_t num_channels,
                            size_t num_samples, double* output) {
    MyDSP::process(input, num_channels, num_samples, output);
}
```

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
- `-d`, `--code <DIR>` Use code from specified directory (copies to `audio/processing/`)
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
2. **Auto-Compilation** (if runtime doesn't exist):
   - Creates local runtime project at `../audio/.playdsp_runtime/`
   - Generates runtime binary from embedded templates
   - Injects your Rust code from `rust_process_audio.rs` (if present)
   - Compiles all C++ files from `processing/` with C++20 (if present)
3. **Audio Processing**:
   - All input formats (16/24/32-bit PCM, 32/64-bit float) converted to f64
   - Runtime binary processes each audio file in 1024-sample buffers
   - Audio normalized to -1.0 to 1.0 range
4. **Output**: Processed files saved as `{filename}_processed_{timestamp}_{rs|cpp}.wav` (32-bit float)

**Recompiling After Code Changes:**
- Delete `../audio/.playdsp_runtime/` to force recompilation
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
- **Optimization**: `-O3` for C++ compilation
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

- Rust toolchain (for building playdsp)
- C++ compiler (for C++ DSP code support)
- Cargo (included with Rust)

## License

MIT License
