use crate::constants::constants::*;
use clap::ArgMatches;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::{Command, Stdio, exit};
use std::time::Duration;
use std::{fs, io};

const CARGO_TOML_TEMPLATE: &str = include_str!("../../templates/Cargo.toml.template");
const BUILD_RS_TEMPLATE: &str = include_str!("../../templates/build.rs.template");
const MAIN_RS_TEMPLATE: &str = include_str!("../../templates/main.rs.template");

pub(crate) fn run_recompile(_matches: &ArgMatches) {
    let audio_dir = Path::new("../audio");
    let runtime_dir = audio_dir.join(".playdsp_runtime");

    let processing_dir = &*PROGRAM_FOLDER;
    if let Err(e) = setup_runtime_project(&runtime_dir, processing_dir) {
        eprintln!("Failed to setup runtime project: {}", e);
        exit(1);
    }

    if let Err(e) = inject_user_rust_code(&runtime_dir, processing_dir) {
        eprintln!("Failed to inject user Rust code: {}", e);
        exit(1);
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Compiling runtime binary...");
    pb.enable_steady_tick(Duration::from_millis(100));

    let compile_start = std::time::Instant::now();

    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&runtime_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run cargo build");

    pb.finish_and_clear();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Failed to compile runtime:\n{}", stderr);
        exit(1);
    }

    println!("Compiled in {:.1}s", compile_start.elapsed().as_secs_f64());
}

fn setup_runtime_project(runtime_dir: &Path, processing_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(runtime_dir)?;
    fs::create_dir_all(runtime_dir.join("src"))?;

    let dependencies = parse_user_dependencies(processing_dir)?;
    let cargo_toml = generate_cargo_toml_with_dependencies(&dependencies);
    fs::write(runtime_dir.join("Cargo.toml"), cargo_toml)?;

    fs::write(runtime_dir.join("build.rs"), BUILD_RS_TEMPLATE)?;
    fs::write(runtime_dir.join("src/main.rs"), MAIN_RS_TEMPLATE)?;

    Ok(())
}

fn inject_user_rust_code(runtime_dir: &Path, processing_dir: &Path) -> io::Result<()> {
    let main_rs_path = runtime_dir.join("src/main.rs");
    let mut main_rs_content = fs::read_to_string(&main_rs_path)?;
    let rust_dir = processing_dir.join("rust");
    let rust_process_file = rust_dir.join("rust_process_audio.rs");

    let runtime_user_code_dir = runtime_dir.join("src/user_code");
    if rust_dir.exists() {
        if runtime_user_code_dir.exists() {
            fs::remove_dir_all(&runtime_user_code_dir)?;
        }

        copy_dir_recursive(&rust_dir, &runtime_user_code_dir)?;

        if rust_process_file.exists() {
            // Create a mod.rs file in user_code directory to make it a proper module
            // Dynamically detect all .rs files and create module declarations
            let mut mod_declarations = Vec::new();

            if let Ok(entries) = fs::read_dir(&runtime_user_code_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                                if file_stem != "mod" {
                                    mod_declarations.push(format!("pub mod {};", file_stem));
                                }
                            }
                        }
                    }
                }
            }

            mod_declarations.sort(); // Ensure consistent ordering
            let mut mod_rs_content = mod_declarations.join("\n");
            mod_rs_content.push_str("\n\npub use rust_process_audio::rust_process;\n");

            fs::write(runtime_user_code_dir.join("mod.rs"), mod_rs_content)?;

            let start_marker =
                "// Rust processing function - will be loaded from user's code\nfn rust_process";
            let end_marker = "\n}\n\n// C++ FFI";

            if let Some(start_idx) = main_rs_content.find(start_marker) {
                if let Some(end_idx) = main_rs_content[start_idx..].find(end_marker) {
                    let actual_end = start_idx + end_idx + 2;
                    main_rs_content.replace_range(
                        start_idx..actual_end,
                        "// Rust processing function - loaded from user's code module\nmod user_code;\n\nfn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {\n    user_code::rust_process(input, output);\n}"
                    );

                    fs::write(&main_rs_path, main_rs_content)?;
                }
            }
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

fn parse_user_dependencies(processing_dir: &Path) -> io::Result<HashMap<String, String>> {
    let mut dependencies = HashMap::new();

    let rust_dir = processing_dir.join("rust");
    let deps_file = rust_dir.join("dependencies.toml");
    if deps_file.exists() {
        if let Ok(content) = fs::read_to_string(&deps_file) {
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
                    if let Some(eq_idx) = line.find('=') {
                        let name = line[..eq_idx].trim().to_string();
                        let value = line[eq_idx + 1..].trim().to_string();
                        dependencies.insert(name, value);
                    }
                }
            }
        }
    }

    if rust_dir.exists() {
        let local_modules = collect_local_modules(&rust_dir);
        scan_rust_dependencies_recursive(&rust_dir, &mut dependencies, &local_modules);
    }

    Ok(dependencies)
}

fn collect_local_modules(dir: &Path) -> HashSet<String> {
    let mut modules = HashSet::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if path.is_dir() {
                    modules.extend(collect_local_modules(&path));
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for line in content.lines() {
                            let line = line.trim();

                            if line.starts_with("mod ") || line.starts_with("pub mod ") {
                                let mod_keyword = if line.starts_with("pub mod ") {
                                    "pub mod "
                                } else {
                                    "mod "
                                };

                                if let Some(rest) = line.strip_prefix(mod_keyword) {
                                    let module_name = rest
                                        .trim_end_matches(';')
                                        .trim()
                                        .split_whitespace()
                                        .next()
                                        .unwrap_or("")
                                        .to_string();

                                    if !module_name.is_empty() {
                                        modules.insert(module_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    modules
}

fn scan_rust_dependencies_recursive(
    dir: &Path,
    dependencies: &mut HashMap<String, String>,
    local_modules: &HashSet<String>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if path.is_dir() {
                    scan_rust_dependencies_recursive(&path, dependencies, local_modules);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let detected = detect_crate_dependencies(&content, local_modules);
                        for crate_name in detected {
                            if !dependencies.contains_key(&crate_name) {
                                dependencies.insert(crate_name.clone(), "\"*\"".to_string());
                            }
                        }
                    }
                }
            }
        }
    }
}

fn detect_crate_dependencies(code: &str, local_modules: &HashSet<String>) -> Vec<String> {
    let mut crates = Vec::new();
    let std_crates = ["std", "core", "alloc"];

    for line in code.lines() {
        let line = line.trim();

        if line.starts_with("use ")
            && !line.starts_with("use crate::")
            && !line.starts_with("use self::")
            && !line.starts_with("use super::")
        {
            if let Some(use_content) = line.strip_prefix("use ") {
                let crate_name = use_content
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(';');

                if !crate_name.is_empty()
                    && !std_crates.contains(&crate_name)
                    && !local_modules.contains(crate_name)
                {
                    if !crates.contains(&crate_name.to_string()) {
                        crates.push(crate_name.to_string());
                    }
                }
            }
        }
    }

    crates
}

fn generate_cargo_toml_with_dependencies(dependencies: &HashMap<String, String>) -> String {
    let mut cargo_toml = CARGO_TOML_TEMPLATE.to_string();

    if !dependencies.is_empty() {
        if let Some(deps_idx) = cargo_toml.find("[dependencies]") {
            let after_deps_header = deps_idx + "[dependencies]".len();
            if let Some(newline_idx) = cargo_toml[after_deps_header..].find('\n') {
                let insert_pos = after_deps_header + newline_idx + 1;

                let mut dep_string = String::new();
                for (name, version) in dependencies {
                    dep_string.push_str(&format!("{} = {}\n", name, version));
                }

                cargo_toml.insert_str(insert_pos, &dep_string);
            }
        }
    }

    cargo_toml
}
