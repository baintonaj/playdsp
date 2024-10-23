# playdsp
Command-Line Audio Signal Processing Framework

This command compiles Rust and/or C++ source files in release mode and processes multiple audio files concurrently, utilizing parallelism for efficient audio processing.

# Usage:
playdsp [OPTIONS] [code_file_path] [audio_file_path]

Arguments:
- [code_file_path]: Optional folder path containing `.cpp` or `.rs` files. If omitted, the default processing folder will be used.
- [audio_file_path]: Optional folder path containing `.wav` files for audio processing. If a single
  folder is provided, all `.wav` files within that folder will be processed.

Options:
- `-r`, `--rust`:     Process with Rust code
- `-c`, `--cpp`:      Process with C++ code
- `-d`, `--code`:     Optional folder path containing .cpp or .rs files
- `-a`, `--audio`:    Optional folder path containing .wav files
- `-h`, `--help`:     Print help
- `-V`, `--version`:  Print version

# Functionality:

This function coordinates the following steps:
1. **Argument Parsing**: Uses Clap to parse command-line arguments, determining the requested processing
   mode (Rust, C++, or both) and file paths.
2. **Source File Compilation**: Depending on the `--rust` or `--cpp` flags, it compiles the respective
   Rust or C++ source files in release mode.
3. **Audio File Loading**: It collects `.wav` files from the specified `audio_file_path` (or the default
   folder if none is provided). If the path is a directory, it retrieves all valid audio files.
4. **Parallel Audio Processing**: Uses the Rayon library to process each audio file concurrently, ensuring
   efficient use of system resources. Each file is processed based on the selected source code (Rust or C++).
5. **File Output**: Processes are saved in the `result/` directory, with filenames indicating whether the
   audio was processed by Rust, C++, or both implementations.

# Error Handling:

The function includes error handling for:
- Invalid or missing file paths.
- Incorrect function signatures in the Rust or C++ files.
- Issues with reading or writing `.wav` files.

In case of errors, appropriate messages will be logged, and the application will **not** attempt to continue processing.

# Examples:

For processing with Rust, replacing the source code and the audio:
```bash
./playdsp --rust --code ../processing  --audio ../replace
```
