use crate::constants::constants::*;
use clap::ArgMatches;
use std::path::Path;
use std::process::exit;
use std::process::Command;
use std::{fs, io};

const CARGO_TOML_TEMPLATE: &str = include_str!("../../templates/Cargo.toml.template");
const BUILD_RS_TEMPLATE: &str = include_str!("../../templates/build.rs.template");
const MAIN_RS_TEMPLATE: &str = include_str!("../../templates/main.rs.template");

pub(crate) fn run_recompile(_matches: &ArgMatches) {
    let audio_dir = Path::new("../audio");
    let runtime_dir = audio_dir.join(".playdsp_runtime");

    println!("Setting up runtime environment...");

    if let Err(e) = setup_runtime_project(&runtime_dir) {
        eprintln!("Failed to setup runtime project: {}", e);
        exit(1);
    }

    println!("Runtime project setup complete.");

    // Check if user has Rust code to include
    let processing_dir = Path::new(PROGRAM_FOLDER);
    if let Err(e) = inject_user_rust_code(&runtime_dir, processing_dir) {
        eprintln!("Failed to inject user Rust code: {}", e);
        exit(1);
    }

    println!("Compiling runtime binary...");
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&runtime_dir)
        .status()
        .expect("Failed to run cargo build");

    if !status.success() {
        eprintln!("Failed to compile runtime");
        exit(1);
    }

    println!("Compilation complete.");
    println!("Runtime binary ready at: {:?}", runtime_dir.join("target/release/playdsp_runtime"));
}

fn setup_runtime_project(runtime_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(runtime_dir)?;
    fs::create_dir_all(runtime_dir.join("src"))?;
    fs::write(runtime_dir.join("Cargo.toml"), CARGO_TOML_TEMPLATE)?;
    fs::write(runtime_dir.join("build.rs"), BUILD_RS_TEMPLATE)?;
    fs::write(runtime_dir.join("src/main.rs"), MAIN_RS_TEMPLATE)?;
    println!("Created runtime project structure at {:?}", runtime_dir);
    Ok(())
}

fn inject_user_rust_code(runtime_dir: &Path, processing_dir: &Path) -> io::Result<()> {
    let main_rs_path = runtime_dir.join("src/main.rs");
    let mut main_rs_content = fs::read_to_string(&main_rs_path)?;
    let rust_process_file = processing_dir.join("rust_process_audio.rs");

    if rust_process_file.exists() {
        let user_code = fs::read_to_string(&rust_process_file)?;
        let start_marker = "// Rust processing function - will be loaded from user's code\nfn rust_process";
        let end_marker = "\n}\n\n// C++ FFI";

        if let Some(start_idx) = main_rs_content.find(start_marker) {
            if let Some(end_idx) = main_rs_content[start_idx..].find(end_marker) {
                let actual_end = start_idx + end_idx + 2;
                main_rs_content.replace_range(
                    start_idx..actual_end,
                    &format!("// Rust processing function - loaded from user's code\n{}", user_code.trim())
                );
                fs::write(&main_rs_path, main_rs_content)?;
                println!("Injected user Rust code from rust_process_audio.rs");
            }
        }
    } else {
        println!("No rust_process_audio.rs found, using default pass-through implementation");
    }
    Ok(())
}