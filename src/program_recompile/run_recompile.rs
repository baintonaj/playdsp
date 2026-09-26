use crate::constants::constants::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::{Command, Stdio, exit};
use std::time::Duration;
use std::{fs, io};

const CARGO_TOML_TEMPLATE: &str = include_str!("../../templates/Cargo.toml.template");
const BUILD_RS_TEMPLATE: &str = include_str!("../../templates/build.rs.template");
const MAIN_RS_TEMPLATE: &str = include_str!("../../templates/main.rs.template");

pub(crate) fn run_recompile() {
    let audio_dir = Path::new("../audio");
    let runtime_dir = audio_dir.join(".playdsp_runtime");

    let processing_dir = &*PROGRAM_FOLDER;
    if let Err(e) = setup_runtime_project(&runtime_dir, processing_dir, false) {
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

pub(crate) fn setup_runtime_project(
    runtime_dir: &Path,
    processing_dir: &Path,
    include_tests: bool,
) -> io::Result<()> {
    fs::create_dir_all(runtime_dir.join("src"))?;

    let dependencies = parse_user_dependencies(processing_dir, include_tests)?;
    let cargo_toml = generate_cargo_toml_with_dependencies(&dependencies);
    write_if_changed(&runtime_dir.join("Cargo.toml"), &cargo_toml)?;
    write_if_changed(&runtime_dir.join("build.rs"), BUILD_RS_TEMPLATE)?;

    Ok(())
}

// Writes main.rs (patched to call the user's Rust code when present) and
// syncs the user's Rust folder into src/user_code/.
pub(crate) fn inject_user_rust_code(runtime_dir: &Path, processing_dir: &Path) -> io::Result<()> {
    let main_rs_path = runtime_dir.join("src/main.rs");
    let mut main_rs_content = normalise_line_endings(MAIN_RS_TEMPLATE);
    let rust_dir = processing_dir.join("rust");
    let rust_process_file = rust_dir.join("rust_process_audio.rs");
    let runtime_user_code_dir = runtime_dir.join("src/user_code");

    if rust_dir.exists() {
        sync_dir(&rust_dir, &runtime_user_code_dir, &["mod.rs"])?;

        if rust_process_file.exists() {
            // Create a mod.rs file in user_code directory to make it a proper module.
            // Dynamically detect all .rs files and create pub module declarations.
            let mut mod_declarations = Vec::new();

            for file_stem in rust_file_stems(&runtime_user_code_dir) {
                if file_stem != "mod" {
                    mod_declarations.push(format!("pub mod {};", file_stem));
                }
            }

            mod_declarations.sort();
            let mut mod_rs_content = mod_declarations.join("\n");
            mod_rs_content.push_str("\n\npub use rust_process_audio::rust_process;\n");

            write_if_changed(&runtime_user_code_dir.join("mod.rs"), &mod_rs_content)?;

            main_rs_content = patch_main_rs(&main_rs_content)?;
        }
    } else if runtime_user_code_dir.exists() {
        fs::remove_dir_all(&runtime_user_code_dir)?;
    }

    write_if_changed(&main_rs_path, &main_rs_content)
}

// A Windows checkout with core.autocrlf gives the embedded template CRLF line
// endings; the LF-only markers in patch_main_rs would then never match.
fn normalise_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

// Replaces the template's pass-through rust_process with one that delegates to
// the user's code. Errors (rather than silently running the pass-through) if
// the markers can't be found.
fn patch_main_rs(template: &str) -> io::Result<String> {
    let start_marker =
        "// Rust processing function - will be loaded from user's code\nfn rust_process";
    let end_marker = "\n}\n\n// C++ FFI";

    let mut content = normalise_line_endings(template);
    let start_idx = content.find(start_marker);
    let end_idx = start_idx.and_then(|start| content[start..].find(end_marker).map(|end| start + end + 2));
    match (start_idx, end_idx) {
        (Some(start), Some(end)) => {
            content.replace_range(
                start..end,
                "// Rust processing function - loaded from user's code module\nmod user_code;\n\nfn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {\n    user_code::rust_process(input, output);\n}",
            );
            Ok(content)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "runtime template is missing the rust_process markers",
        )),
    }
}

// Only touching files whose content changed keeps their mtimes stable, so
// cargo can skip the rebuild (and the C++ recompile) when nothing changed.
pub(crate) fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    if fs::read(path).map(|existing| existing == content.as_bytes()).unwrap_or(false) {
        return Ok(());
    }
    fs::write(path, content)
}

// Mirrors src into dst: copies new or changed files and removes files that no
// longer exist in src. Names in `keep` (generated files) are left alone.
fn sync_dir(src: &Path, dst: &Path, keep: &[&str]) -> io::Result<()> {
    fs::create_dir_all(dst)?;

    let mut source_names = HashSet::new();
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        source_names.insert(entry.file_name());

        if path.is_dir() {
            if dest_path.is_file() {
                fs::remove_file(&dest_path)?;
            }
            sync_dir(&path, &dest_path, &[])?;
        } else {
            if dest_path.is_dir() {
                fs::remove_dir_all(&dest_path)?;
            }
            let contents = fs::read(&path)?;
            if fs::read(&dest_path).map(|existing| existing != contents).unwrap_or(true) {
                fs::write(&dest_path, contents)?;
            }
        }
    }

    for entry in fs::read_dir(dst)? {
        let entry = entry?;
        let name = entry.file_name();
        if source_names.contains(&name) || keep.iter().any(|k| name == *k) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

fn rust_file_stems(dir: &Path) -> Vec<String> {
    let mut stems = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    stems.push(stem.to_string());
                }
            }
        }
    }
    stems
}

fn parse_user_dependencies(
    processing_dir: &Path,
    include_tests: bool,
) -> io::Result<HashMap<String, String>> {
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
        // Top-level files are declared as modules by the generated mod.rs.
        let mut local_modules = collect_local_modules(&rust_dir);
        local_modules.extend(rust_file_stems(&rust_dir));
        scan_rust_dependencies_recursive(&rust_dir, &mut dependencies, &local_modules);

        let tests_dir = processing_dir.join("tests");
        if include_tests && tests_dir.exists() {
            local_modules.extend(rust_file_stems(&tests_dir));
            scan_rust_dependencies_recursive(&tests_dir, &mut dependencies, &local_modules);
        }
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
    let non_crates = ["std", "core", "alloc", "crate", "self", "super"];

    for line in code.lines() {
        let mut line = line.trim();

        // Strip visibility: `pub use`, `pub(crate) use`, `pub(super) use`, ...
        if let Some(rest) = line.strip_prefix("pub") {
            let rest = rest.trim_start();
            line = if let Some(after_paren) = rest.strip_prefix('(') {
                match after_paren.find(')') {
                    Some(close) => after_paren[close + 1..].trim_start(),
                    None => continue,
                }
            } else {
                rest
            };
        }

        let path = if let Some(rest) = line.strip_prefix("use ") {
            rest
        } else if let Some(rest) = line.strip_prefix("extern crate ") {
            rest
        } else {
            continue;
        };

        // `use ::foo::Bar` is an absolute path to crate `foo`.
        let path = path.trim_start().trim_start_matches("::");
        // The crate name is the leading identifier: stops at `::`, ` as `, `;`, `{`, ...
        let crate_name: String = path
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        if !crate_name.is_empty()
            && !non_crates.contains(&crate_name.as_str())
            && !local_modules.contains(&crate_name)
            && !crates.contains(&crate_name)
        {
            crates.push(crate_name);
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

                // Sorted so the generated Cargo.toml is byte-identical between runs.
                let mut sorted: Vec<_> = dependencies.iter().collect();
                sorted.sort();

                let mut dep_string = String::new();
                for (name, version) in sorted {
                    dep_string.push_str(&format!("{} = {}\n", name, version));
                }

                cargo_toml.insert_str(insert_pos, &dep_string);
            }
        }
    }

    cargo_toml
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(code: &str) -> Vec<String> {
        detect_crate_dependencies(code, &HashSet::from(["filters".to_string()]))
    }

    #[test]
    fn patches_template_with_lf_and_crlf_line_endings() {
        let lf = patch_main_rs(MAIN_RS_TEMPLATE).unwrap();
        let crlf = patch_main_rs(&MAIN_RS_TEMPLATE.replace('\n', "\r\n")).unwrap();
        assert_eq!(lf, crlf);
        assert!(lf.contains("mod user_code;"));
        assert!(lf.contains("user_code::rust_process(input, output);"));
        assert!(!lf.contains("will be loaded from user's code"));
        assert!(lf.contains("\n}\n\n// C++ FFI"), "delegate must end right before the C++ FFI section");
    }

    #[test]
    fn patch_fails_loudly_without_markers() {
        assert!(patch_main_rs("fn main() {}").is_err());
    }

    #[test]
    fn detects_plain_and_grouped_imports() {
        assert_eq!(
            detect("use rustfft::FftPlanner;\nuse rand::{Rng, thread_rng};"),
            vec!["rustfft", "rand"]
        );
    }

    #[test]
    fn handles_alias_visibility_absolute_and_extern_crate() {
        assert_eq!(detect("use num_complex as nc;"), vec!["num_complex"]);
        assert_eq!(detect("pub use biquad::Biquad;"), vec!["biquad"]);
        assert_eq!(detect("pub(crate) use dasp::Sample;"), vec!["dasp"]);
        assert_eq!(detect("use ::realfft::RealFftPlanner;"), vec!["realfft"]);
        assert_eq!(detect("extern crate libm;"), vec!["libm"]);
        assert_eq!(detect("use serde;"), vec!["serde"]);
    }

    #[test]
    fn ignores_std_relative_and_local_modules() {
        let code = "use std::sync::Mutex;\nuse core::f64;\nuse crate::x;\nuse self::y;\nuse super::z;\nuse filters::Lowpass;";
        assert!(detect(code).is_empty());
    }

    #[test]
    fn ignores_non_use_lines_and_deduplicates() {
        let code = "// use fake::Thing;\nlet user = 1;\nuse rand::Rng;\nuse rand::random;";
        assert_eq!(detect(code), vec!["rand"]);
    }
}
