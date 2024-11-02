mod file_processing;
use file_processing::code_processing::create_folders_and_copy_files::*;
use file_processing::code_processing::process_and_copy_files::*;
use file_processing::code_processing::get_program_files::*;
use file_processing::audio_processing::replace_audio_files::*;
use file_processing::audio_processing::get_audio_files_from_folder::*;
use signal_processing::process_multiple_audio_files::*;
mod constants;
mod program_recompile;
mod signal_processing;
mod external_processing;

use program_recompile::run_recompile::*;

use constants::constants::*;
use clap::{Arg, ArgAction, Command};

fn main() {
    let matches = Command::new("playdsp")
        .version("0.1.0")
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
        .arg(Arg::new("rust")
            .short('r')
            .long("rust")
            .required(false)
            .num_args(0)
            .action(ArgAction::Set)
            .help("Process with Rust code"))
        .arg(Arg::new("cpp")
            .short('c')
            .long("cpp")
            .required(false)
            .num_args(0)
            .action(ArgAction::Set)
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
        .get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("new") {
        let dot = &".".to_string();
        let base_dir = sub_matches.get_one::<String>("dir").unwrap_or(dot);
        create_folders_and_copy_files(base_dir);
        return;
    }

    let rust_present = matches.contains_id("rust");
    let cpp_present = matches.contains_id("cpp");

    if let Some(folder_path) = matches.get_one::<String>(CODE_FILE_PATH_NAME) {
        if rust_present && !cpp_present {
            if let Err(e) = process_and_copy_files(folder_path, "rust") {
                eprintln!("Error processing folder for Rust: {}", e);
                return;
            }
        } else if cpp_present && !rust_present {
            if let Err(e) = process_and_copy_files(folder_path, "cpp") {
                eprintln!("Error processing folder for C++: {}", e);
                return;
            }
        } else if !rust_present && !cpp_present {
            if let Err(e) = process_and_copy_files(folder_path, "both") {
                eprintln!("Error processing folder for both Rust and C++: {}", e);
                return;
            }
        }

        run_recompile(&matches);
    }

    if let Some(input_folder) = matches.get_one::<String>(AUDIO_FILE_PATH_NAME) {
        if let Err(e) = replace_audio_files(input_folder) {
            eprintln!("Error replacing audio files: {}", e);
            return;
        }
    }

    if !rust_present && !cpp_present {
        println!("Processing with both Rust and C++ code");
    } else if rust_present {
        println!("Processing with Rust code");
    } else if cpp_present {
        println!("Processing with C++ code");
    }

    let audio_files_to_process = get_audio_files_from_folder(SOURCE_NAME);

    let mut rust_files: Vec<String> = vec![];
    let mut cpp_files: Vec<String> = vec![];

    if !rust_present && !cpp_present {
        rust_files = get_program_files(&*PROGRAM_FOLDER, "rs");
        cpp_files = get_program_files(&*PROGRAM_FOLDER, "cpp");
    } else if rust_present {
        rust_files = get_program_files(&*PROGRAM_FOLDER, "rs");
    } else if cpp_present {
        cpp_files = get_program_files(&*PROGRAM_FOLDER, "cpp");
    }

    if !rust_present && !cpp_present {
        println!("Rust files: {:?}", rust_files);
        println!("C++ files: {:?}", cpp_files);
        let mut all_files = Vec::new();
        all_files.append(rust_files.as_mut());
        all_files.append(cpp_files.as_mut());
        process_multiple_audio_files(audio_files_to_process, all_files);
    } else if rust_present {
        println!("Rust files: {:?}", rust_files);
        process_multiple_audio_files(audio_files_to_process, rust_files);
    } else if cpp_present {
        println!("C++ files: {:?}", cpp_files);
        process_multiple_audio_files(audio_files_to_process, cpp_files);
    }

}
