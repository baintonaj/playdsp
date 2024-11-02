
# playdsp
Command-Line Audio Signal Processing Framework

This command compiles Rust and/or C++ source files in release mode and processes multiple audio files concurrently, utilizing parallelism for efficient audio processing.

## Usage:
`playdsp [OPTIONS] [code_file_path] [audio_file_path]`

### Arguments:
- **[code_file_path]**: Optional folder path containing `.cpp` or `.rs` files. If omitted, the default processing folder will be used.
- **[audio_file_path]**: Optional folder path containing `.wav` files for audio processing. If a single folder is provided, all `.wav` files within that folder will be processed.

### Options:
- `-r`, `--rust`:     Process with Rust code
- `-c`, `--cpp`:      Process with C++ code
- `-d`, `--code`:     Optional folder path containing .cpp or .rs files
- `-a`, `--audio`:    Optional folder path containing `.wav` files
- `-h`, `--help`:     Print help
- `-V`, `--version`:  Print version

### Subcommands:
- **new**: Creates a new folder structure for DSP processing:
  ```bash
  playdsp new [--dir <base_dir>]
  ```
    - Creates the following folder structure:
      ```
      <base_dir>/
            audio/
                processing/
                result/
                source/
      ```
    - `--dir`: Optional base directory to create the `audio/` folder. If not specified, the current directory is used.

## Functionality:

This command coordinates the following steps:
1. **Argument Parsing**: Uses Clap to parse command-line arguments, determining the requested processing mode (Rust, C++, or both) and file paths.
2. **Folder Creation (new subcommand)**: Creates a new folder structure with subdirectories for processing files (`processing/`), audio sources (`source/`), and processed results (`result/`).
3. **Source File Compilation**: Depending on the `--rust` or `--cpp` flags, it compiles the respective Rust or C++ source files in release mode.
4. **Audio File Loading**: It collects `.wav` files from the specified `audio_file_path` (or the default folder if none is provided). If the path is a directory, it retrieves all valid audio files.
5. **Parallel Audio Processing**: Uses the Rayon library to process each audio file concurrently, ensuring efficient use of system resources. Each file is processed based on the selected source code (Rust or C++).
6. **File Output**: Processed files are saved in the `result/` directory, with filenames indicating whether the audio was processed by Rust, C++, or both implementations.

## Error Handling:

The function includes error handling for:
- Invalid or missing file paths.
- Incorrect function signatures in the Rust or C++ files.
- Issues with reading or writing `.wav` files.

In case of errors, appropriate messages will be logged, and the application will not attempt to continue processing.

## Examples:

To create a new folder structure for DSP processing in the current directory:
```bash
playdsp new
```

To create the folder structure in a specific directory:
```bash
playdsp new --dir /path/to/base
```

For processing with Rust, **replacing** the source code and the audio:
```bash
sudo playdsp --rust --code ../path/to/code_processing --audio ../path/to/audio_replace
```

For processing with both Rust and C++ code, **replacing** the source code and the audio:
```bash
sudo playdsp --code ../path/to/code_processing --audio ../path/to/audio_replace
```

For processing with both Rust and C++ code, **maintaining** the source code and audio:
```bash
playdsp
```