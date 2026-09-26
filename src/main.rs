mod file_processing;

use file_processing::audio_processing::get_audio_files_from_folder::*;
use file_processing::audio_processing::replace_audio_files::*;
use file_processing::code_processing::create_folders_and_copy_files::*;
use file_processing::code_processing::get_program_files::*;
use file_processing::code_processing::process_and_copy_files::*;
use signal_processing::process_multiple_audio_files::*;
mod constants;
mod program_recompile;
mod signal_processing;

use program_recompile::run_recompile::*;
use program_recompile::run_tests::*;

use clap::{Arg, ArgAction, Command};
use constants::constants::*;

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
        )
        .build_global()
        .ok();

    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Andy Bainton <baintonaj@gmail.com>")
        .about("Compiles Rust and/or C++ files in release mode, and processes multiple audio files concurrently")
        .subcommand(
            Command::new("new")
                .about("Creates new folder structure for DSP processing")
                .arg(
                    Arg::new("dir")
                        .short('d')
                        .long("dir")
                        .help("Optional base directory to create subfolders")
                        .required(false)
                        .num_args(1)
                        .action(ArgAction::Set)
                )
        )
        .subcommand(
            Command::new("test")
                .about("Compile and run DSP tests from audio/processing/tests/")
                .arg(Arg::new("rust")
                    .short('r')
                    .long("rust")
                    .required(false)
                    .num_args(0)
                    .action(ArgAction::SetTrue)
                    .help("Run only Rust DSP tests (files not prefixed with cpp_)"))
                .arg(Arg::new("cpp")
                    .short('c')
                    .long("cpp")
                    .required(false)
                    .num_args(0)
                    .action(ArgAction::SetTrue)
                    .help("Run only C++ DSP tests (files prefixed with cpp_)"))
        )
        .arg(Arg::new("rust")
            .short('r')
            .long("rust")
            .required(false)
            .num_args(0)
            .action(ArgAction::SetTrue)
            .help("Process with Rust code"))
        .arg(Arg::new("cpp")
            .short('c')
            .long("cpp")
            .required(false)
            .num_args(0)
            .action(ArgAction::SetTrue)
            .help("Process with C++ code"))
        .arg(Arg::new(CODE_FILE_PATH_NAME)
            .short('d')
            .long("code")
            .help("Optional folder path containing .cpp or .rs files")
            .required(false)
            .num_args(1)
            .action(ArgAction::Set))
        .arg(Arg::new(AUDIO_FILE_PATH_NAME)
            .short('a')
            .long("audio")
            .help("Optional folder path containing .wav files")
            .required(false)
            .num_args(1)
            .action(ArgAction::Set))
        .arg(Arg::new("meta")
            .short('m')
            .long("meta")
            .required(false)
            .num_args(0)
            .action(ArgAction::SetTrue)
            .help("Preserve BWF metadata (bext chunk) from input WAV files in output"))
        .get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("new") {
        let dot = &".".to_string();
        let base_dir = sub_matches.get_one::<String>("dir").unwrap_or(dot);
        create_folders_and_copy_files(base_dir);
        return;
    }

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let rust_only = test_matches.get_flag("rust");
        let cpp_only = test_matches.get_flag("cpp");
        run_tests(rust_only, cpp_only);
        return;
    }

    // Neither flag, or both flags, means process with both languages.
    let rust_flag = matches.get_flag("rust");
    let cpp_flag = matches.get_flag("cpp");
    let use_rust = rust_flag || !cpp_flag;
    let use_cpp = cpp_flag || !rust_flag;
    let preserve_meta = matches.get_flag("meta");

    if let Some(folder_path) = matches.get_one::<String>(CODE_FILE_PATH_NAME) {
        let file_type = match (use_rust, use_cpp) {
            (true, false) => "rust",
            (false, true) => "cpp",
            _ => "both",
        };
        if let Err(e) = process_and_copy_files(folder_path, file_type) {
            eprintln!("Error processing code folder '{}': {}", folder_path, e);
            return;
        }
    }

    if let Some(input_folder) = matches.get_one::<String>(AUDIO_FILE_PATH_NAME) {
        if let Err(e) = replace_audio_files(input_folder) {
            eprintln!("Error replacing audio files: {}", e);
            return;
        }
    }

    let runtime_binary = std::path::PathBuf::from("../audio/.playdsp_runtime/target/release")
        .join(format!("playdsp_runtime{}", std::env::consts::EXE_SUFFIX));

    let rust_dir = RUST_FOLDER.as_path();
    let cpp_dir = CPP_FOLDER.as_path();

    let has_rust_files = rust_dir.join("rust_process_audio.rs").exists();
    let has_dependencies_toml = rust_dir.join("dependencies.toml").exists();
    let has_cpp_code = has_cpp_files(cpp_dir);

    // Compiled once per run. Unchanged sources make this a fast cargo no-op.
    if has_rust_files || has_cpp_code || has_dependencies_toml {
        println!("DSP code detected - recompiling runtime to ensure latest changes...");
        run_recompile();
    } else if !runtime_binary.exists() {
        println!("Runtime binary not found. Compiling runtime with default code...");
        run_recompile();
    }

    let audio_files_to_process = get_audio_files_from_folder(&SOURCE_FOLDER);
    if audio_files_to_process.is_empty() {
        println!("No .wav files found in {}", SOURCE_FOLDER.display());
        return;
    }

    let mut program_files: Vec<String> = vec![];
    if use_rust {
        program_files.extend(get_program_files(RUST_FOLDER.to_str().unwrap_or(""), "rs"));
    }
    if use_cpp {
        program_files.extend(get_program_files(CPP_FOLDER.to_str().unwrap_or(""), "cpp"));
    }

    match (use_rust, use_cpp) {
        (true, false) => println!("Processing with Rust code"),
        (false, true) => println!("Processing with C++ code"),
        _ => println!("Processing with both Rust and C++ code"),
    }

    if program_files.is_empty() {
        println!("No DSP entry files found (rust_process_audio.rs / cpp_process_audio.cpp)");
        return;
    }

    process_multiple_audio_files(&audio_files_to_process, &program_files, preserve_meta);
}
