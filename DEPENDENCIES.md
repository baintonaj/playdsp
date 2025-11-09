# Managing Rust Dependencies in PlayDSP

PlayDSP automatically handles external crate dependencies in your Rust DSP code. The system recursively scans all `.rs` files in your `rust/` folder and supports both automatic detection and explicit version control.

## Automatic Dependency Detection

When you use external crates in any `.rs` file in your `rust/` folder, PlayDSP will automatically detect them from your `use` statements and add them to the runtime's Cargo.toml.

**Example:**

```rust
// In ../audio/processing/rust/rust_process_audio.rs
use rand::Rng;
use num_complex::Complex;

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    // Your DSP code using rand and num_complex
}
```

Or in any other module file:
```rust
// In ../audio/processing/rust/filters/biquad.rs
use ndarray::Array1;

pub struct Biquad {
    // ...
}
```

When you run `playdsp`, it will automatically:
1. Recursively scan all `.rs` files in the `rust/` folder
2. Detect `rand`, `num_complex`, and `ndarray` from the use statements
3. Add them to the runtime Cargo.toml with the latest version (`"*"`)
4. Compile the runtime with these dependencies

## Explicit Version Control (Recommended)

For production code or when you need specific versions, create a `dependencies.toml` file in your `rust/` folder:

**File:** `../audio/processing/rust/dependencies.toml`

```toml
[dependencies]
rand = "0.8"
num-complex = "0.4"
rustfft = { version = "6.0", features = ["avx"] }
```

This approach gives you:
- **Version control**: Pin specific versions for reproducible builds
- **Feature flags**: Enable specific crate features
- **Complex configurations**: Use full Cargo.toml dependency syntax

## How It Works

1. **Priority**: Explicit `dependencies.toml` entries take precedence over auto-detected ones
2. **Merging**: Auto-detected crates not in `dependencies.toml` are added with `"*"` version
3. **Standard library**: `std`, `core`, and `alloc` are automatically excluded

## Complete Example

**Structure:**
```
audio/
├── processing/
│   ├── rust/
│   │   ├── rust_process_audio.rs
│   │   ├── my_dsp.rs
│   │   ├── filters/
│   │   │   ├── mod.rs
│   │   │   └── biquad.rs
│   │   └── dependencies.toml  (optional)
│   └── cpp/
│       └── cpp_process_audio.cpp
├── source/
└── result/
```

**rust_process_audio.rs:**
```rust
use rand::Rng;
use rand::distributions::Uniform;
use rustfft::{FftPlanner, num_complex::Complex};

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let mut rng = rand::thread_rng();
    let dist = Uniform::new(-0.1, 0.1);

    // Add some noise
    for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
        for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
            *out_sample = *in_sample + rng.sample(dist);
        }
    }
}
```

**dependencies.toml:**
```toml
[dependencies]
rand = "0.8"
rustfft = { version = "6.0", features = ["avx"] }
```

## Output

When you run `playdsp`, you'll see:

```
Setting up runtime environment...
Found dependencies.toml with 2 explicit dependencies
Created runtime project structure at "../audio/.playdsp_runtime"
Added 2 user dependencies to runtime Cargo.toml:
  rand = "0.8"
  rustfft = { version = "6.0", features = ["avx"] }
Runtime project setup complete.
Compiling runtime binary...
```

## Notes

- Dependencies are resolved fresh on every run when code exists in `../audio/processing/rust/`
- The system recursively scans all `.rs` files in all subdirectories for dependency detection
- The system automatically excludes standard library crates (std, core, alloc)
- Dependencies are added to the runtime's Cargo.toml, not your main PlayDSP binary
- If compilation fails due to incompatible versions, update your `rust/dependencies.toml` with compatible versions
- Place `dependencies.toml` directly in the `rust/` folder, not in subdirectories
