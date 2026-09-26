use crate::constants::constants::*;
use crate::file_processing::code_processing::get_program_files::has_cpp_files;
use std::path::Path;
use std::process::{Command, Stdio, exit};
use std::{fs, io};

use super::run_recompile::{inject_user_rust_code, setup_runtime_project};

pub(crate) fn run_tests(rust_only: bool, cpp_only: bool) {
    let audio_dir = Path::new("../audio");
    let runtime_dir = audio_dir.join(".playdsp_runtime");
    let processing_dir = &*PROGRAM_FOLDER;

    println!("DSP code detected - recompiling for test...");

    if let Err(e) = setup_runtime_project(&runtime_dir, processing_dir, true) {
        eprintln!("Failed to setup runtime project: {}", e);
        exit(1);
    }

    if let Err(e) = inject_user_rust_code(&runtime_dir, processing_dir) {
        eprintln!("Failed to inject user Rust code: {}", e);
        exit(1);
    }

    if let Err(e) = inject_test_files(&runtime_dir, rust_only, cpp_only) {
        eprintln!("Failed to inject test files: {}", e);
        exit(1);
    }

    // Single-threaded: DSP state is a global singleton, so parallel tests would
    // interleave calls into the same filter/delay state and become flaky.
    let status = Command::new("cargo")
        .arg("test")
        .arg("--")
        .arg("--test-threads=1")
        .current_dir(&runtime_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to run cargo test");

    exit(status.code().unwrap_or(1));
}

// Copies test files from TESTS_FOLDER into user_code/ and appends
// #[cfg(test)] mod declarations to mod.rs.
//
// Naming convention: files prefixed with "cpp_" are C++ tests; all others
// are Rust tests. This controls which files are copied when --rust or --cpp
// is passed.
fn inject_test_files(runtime_dir: &Path, rust_only: bool, cpp_only: bool) -> io::Result<()> {
    let tests_dir = &*TESTS_FOLDER;
    if !tests_dir.exists() {
        return Ok(());
    }

    let runtime_user_code_dir = runtime_dir.join("src/user_code");
    if !RUST_FOLDER.join("rust_process_audio.rs").exists() {
        // No Rust DSP code was injected, so there is no user_code module to attach tests to.
        eprintln!("No rust_process_audio.rs found in processing/rust/ - skipping test injection");
        return Ok(());
    }
    // cpp_process() only exists when C++ sources were compiled.
    let cpp_available = has_cpp_files(&CPP_FOLDER);
    let mut test_mod_names: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(tests_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                let stem = match path.file_stem().and_then(|s| s.to_str()) {
                    Some(s) => s.to_string(),
                    None => continue,
                };

                let is_cpp_test = stem.starts_with("cpp_");
                if rust_only && is_cpp_test {
                    continue;
                }
                if cpp_only && !is_cpp_test {
                    continue;
                }
                if is_cpp_test && !cpp_available {
                    println!("Skipping {}: no C++ code in processing/cpp/", entry.file_name().to_string_lossy());
                    continue;
                }

                fs::copy(&path, runtime_user_code_dir.join(entry.file_name()))?;
                test_mod_names.push(stem);
            }
        }
    }

    if !test_mod_names.is_empty() {
        test_mod_names.sort();
        let mod_rs_path = runtime_user_code_dir.join("mod.rs");
        let mut mod_rs_content = fs::read_to_string(&mod_rs_path)?;
        mod_rs_content.push_str("\n// --- test modules (injected by playdsp) ---\n");
        for name in &test_mod_names {
            mod_rs_content.push_str(&format!("#[cfg(test)] mod {};\n", name));
        }
        fs::write(&mod_rs_path, mod_rs_content)?;
    }

    Ok(())
}
