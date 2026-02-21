# Installation Guide

## Building playdsp

```bash
cd /path/to/playdsp
cargo build --release
```

The binary will be created at: `target/release/playdsp`

## System-Wide Installation

You have several options for installing playdsp system-wide:

### Option 1: Copy Binary (Recommended)

```bash
sudo cp target/release/playdsp /usr/local/bin/
```

Verify installation:
```bash
playdsp --version
```

### Option 2: Symbolic Link

```bash
sudo ln -s $(pwd)/target/release/playdsp /usr/local/bin/playdsp
```

### Option 3: Cargo Install

```bash
cargo install --path .
```

This installs to `~/.cargo/bin/playdsp` (ensure `~/.cargo/bin` is in your PATH)

To update after code changes:
```bash
cargo install --path . --force
```

## Uninstallation

### If installed via Option 1 (Copy):
```bash
sudo rm /usr/local/bin/playdsp
```

### If installed via Option 2 (Symbolic Link):
```bash
sudo rm /usr/local/bin/playdsp
```

### If installed via Option 3 (Cargo):
```bash
cargo uninstall playdsp
```

## Updating

To update playdsp after pulling new changes:

### Option A: One-step reinstall (recommended)

```bash
sudo make reinstall
```

This runs `make clean`, `make release`, and `make install` in sequence. Pass `DESTDIR` if you installed to a non-default location:

```bash
sudo make reinstall DESTDIR=/usr/local/bin
```

### Option B: Manual steps

1. Rebuild the binary:
```bash
cargo build --release
```

2. Reinstall using your preferred method:
```bash
sudo cp target/release/playdsp /usr/local/bin/
```
or
```bash
cargo install --path . --force
```

3. Force recompilation of runtime on next run:
```bash
rm -rf ../audio/.playdsp_runtime
```

4. Verify the version:
```bash
playdsp --version
```

## Requirements

- **Rust toolchain 1.85+**: Required for edition 2024. Install from [rustup.rs](https://rustup.rs/)
  - Check your version: `rustc --version`
  - Update if needed: `rustup update stable`
- **C++ compiler**: Required for C++ DSP code compilation
  - macOS: Install Xcode Command Line Tools (`xcode-select --install`)
  - Linux: Install `build-essential` (Debian/Ubuntu) or `base-devel` (Arch)
  - Windows: Install MSVC Build Tools or MinGW

## Verifying Installation

After installation, verify everything works:

```bash
playdsp --version

cd /tmp
playdsp new

ls audio/
ls audio/processing/
ls audio/processing/rust/
ls audio/processing/cpp/

playdsp --help
```

You should see the folder structure with `rust/` and `cpp/` subdirectories containing the entry point files.

## Troubleshooting

### "playdsp: command not found"

- Verify `/usr/local/bin` is in your PATH:
  ```bash
  echo $PATH
  ```

- If using cargo install, ensure `~/.cargo/bin` is in your PATH:
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"
  ```

### "Failed to compile runtime"

- Ensure you have a C++ compiler installed
- Check that you're running playdsp from a directory where `../audio` can be created
- Delete `../audio/.playdsp_runtime` and try again

### "error: package `playdsp` cannot be built because it requires rustc 1.85..."
- Run `rustup update stable` to update to the latest Rust toolchain
- Verify with `rustc --version` that you have 1.85.0 or newer

### Version not updating after reinstall

- If using cargo install, use the `--force` flag
- Clear cargo cache: `rm -rf ~/.cargo/registry/cache`
- Verify you're running the correct binary: `which playdsp`

## Platform Notes

### macOS
- Xcode Command Line Tools required for C++ compilation
- /usr/local/bin should be in PATH by default

### Linux
- Ensure gcc/g++ is installed for C++ compilation
- /usr/local/bin should be in PATH by default

### Windows
- Install MSVC Build Tools (recommended) or MinGW-w64
- MSVC Build Tools: available from [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- MSVC is the recommended C++ compiler on Windows — playdsp uses MSVC-specific flags (`/O2`, `/std:c++20`, `/EHsc`) when building with MSVC
- MinGW users: ensure `g++` is in PATH and add the MinGW bin directory to PATH
- The runtime binary is automatically detected as `playdsp_runtime.exe` on Windows

## Quick Install Script

```bash
cargo build --release && sudo cp target/release/playdsp /usr/local/bin/
```
