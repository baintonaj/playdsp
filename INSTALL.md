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

## Requirements

- **Rust toolchain**: Install from [rustup.rs](https://rustup.rs/)
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
- Install MSVC Build Tools or MinGW-w64
- Add installation directory to PATH manually
- Runtime directory will be created at `../audio/.playdsp_runtime`

## Quick Install Script

```bash
cargo build --release && sudo cp target/release/playdsp /usr/local/bin/
```
