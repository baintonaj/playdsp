use crate::constants::constants::*;
use clap::ArgMatches;
use std::collections::HashMap;
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

    let processing_dir = Path::new(PROGRAM_FOLDER);
    if let Err(e) = setup_runtime_project(&runtime_dir, processing_dir) {
        eprintln!("Failed to setup runtime project: {}", e);
        exit(1);
    }

    println!("Runtime project setup complete.");

    // Check if user has Rust code to include
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

fn setup_runtime_project(runtime_dir: &Path, processing_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(runtime_dir)?;
    fs::create_dir_all(runtime_dir.join("src"))?;

    // Parse user dependencies and generate Cargo.toml with them
    let dependencies = parse_user_dependencies(processing_dir)?;
    let cargo_toml = generate_cargo_toml_with_dependencies(&dependencies);
    fs::write(runtime_dir.join("Cargo.toml"), cargo_toml)?;

    fs::write(runtime_dir.join("build.rs"), BUILD_RS_TEMPLATE)?;
    fs::write(runtime_dir.join("src/main.rs"), MAIN_RS_TEMPLATE)?;
    println!("Created runtime project structure at {:?}", runtime_dir);

    if !dependencies.is_empty() {
        println!("Added {} user dependencies to runtime Cargo.toml:", dependencies.len());
        for (name, version) in dependencies.iter() {
            println!("  {} = \"{}\"", name, version);
        }
    }

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

/// Parse user dependencies from both rust_process_audio.rs and optional dependencies.toml
fn parse_user_dependencies(processing_dir: &Path) -> io::Result<HashMap<String, String>> {
    let mut dependencies = HashMap::new();

    // First, check for explicit dependencies.toml file
    let deps_file = processing_dir.join("dependencies.toml");
    if deps_file.exists() {
        if let Ok(content) = fs::read_to_string(&deps_file) {
            // Simple TOML parsing for [dependencies] section
            let mut in_dependencies_section = false;
            for line in content.lines() {
                let line = line.trim();
                if line == "[dependencies]" {
                    in_dependencies_section = true;
                    continue;
                }
                if line.starts_with('[') && line.ends_with(']') {
                    in_dependencies_section = false;
                    continue;
                }
                if in_dependencies_section && !line.is_empty() && !line.starts_with('#') {
                    // Parse lines like: crate_name = "version" or crate_name = { version = "1.0", features = ["foo"] }
                    if let Some(eq_idx) = line.find('=') {
                        let name = line[..eq_idx].trim().to_string();
                        let value = line[eq_idx + 1..].trim().to_string();
                        dependencies.insert(name, value);
                    }
                }
            }
            println!("Found dependencies.toml with {} explicit dependencies", dependencies.len());
        }
    }

    // Second, scan rust_process_audio.rs for external crate usage
    let rust_file = processing_dir.join("rust_process_audio.rs");
    if rust_file.exists() {
        if let Ok(content) = fs::read_to_string(&rust_file) {
            let detected = detect_crate_dependencies(&content);
            for crate_name in detected {
                // Only add if not already specified in dependencies.toml
                if !dependencies.contains_key(&crate_name) {
                    // Use wildcard version for auto-detected crates
                    dependencies.insert(crate_name.clone(), "\"*\"".to_string());
                    println!("Auto-detected dependency: {} (using latest version)", crate_name);
                }
            }
        }
    }

    Ok(dependencies)
}

/// Detect external crate dependencies from use statements
fn detect_crate_dependencies(code: &str) -> Vec<String> {
    let mut crates = Vec::new();
    let std_crates = ["std", "core", "alloc"];

    for line in code.lines() {
        let line = line.trim();

        // Match patterns like: use crate_name::...
        if line.starts_with("use ") && !line.starts_with("use crate::") && !line.starts_with("use self::") && !line.starts_with("use super::") {
            if let Some(use_content) = line.strip_prefix("use ") {
                // Extract the root crate name (first segment before ::)
                let crate_name = use_content
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(';');

                // Skip standard library crates
                if !crate_name.is_empty() && !std_crates.contains(&crate_name) {
                    if !crates.contains(&crate_name.to_string()) {
                        crates.push(crate_name.to_string());
                    }
                }
            }
        }
    }

    crates
}

/// Generate Cargo.toml content with user dependencies injected
fn generate_cargo_toml_with_dependencies(dependencies: &HashMap<String, String>) -> String {
    let mut cargo_toml = CARGO_TOML_TEMPLATE.to_string();

    // If there are user dependencies, add them to the [dependencies] section
    if !dependencies.is_empty() {
        // Find the [dependencies] section and insert after it
        if let Some(deps_idx) = cargo_toml.find("[dependencies]") {
            // Find the end of the line after [dependencies]
            let after_deps_header = deps_idx + "[dependencies]".len();
            if let Some(newline_idx) = cargo_toml[after_deps_header..].find('\n') {
                let insert_pos = after_deps_header + newline_idx + 1;

                // Build the dependency string
                let mut dep_string = String::new();
                for (name, version) in dependencies {
                    dep_string.push_str(&format!("{} = {}\n", name, version));
                }

                // Insert after the existing dependencies
                cargo_toml.insert_str(insert_pos, &dep_string);
            }
        }
    }

    cargo_toml
}