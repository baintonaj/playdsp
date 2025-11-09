use std::path::Path;
use std::process::Command;
use rayon::prelude::*;
use chrono::Local;
use crate::constants::constants::*;

pub(crate) fn process_multiple_audio_files(audio_files: &Vec<String>, program_paths: &Vec<String>) {
    let runtime_binary = Path::new("../audio/.playdsp_runtime/target/release/playdsp_runtime");

    if !runtime_binary.exists() {
        eprintln!("Runtime binary not found. This shouldn't happen after recompilation.");
        return;
    }

    audio_files.par_iter().for_each(|audio_file| {
        let current_time = Local::now().format("%Y_%m_%d_%H_%M").to_string();
        let audio_stem = Path::new(audio_file).file_stem().unwrap().to_str().unwrap();

        program_paths.par_iter().for_each(|program_path| {
            let program_suffix  = Path::new(program_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if Path::new(&program_path).exists() {
                let output_file = format!("{}/{}_processed_{}_{}.wav", RESULT_FOLDER, audio_stem, current_time, program_suffix);

                let status = Command::new(&runtime_binary)
                    .arg(audio_file)
                    .arg(&output_file)
                    .arg(program_suffix)
                    .status();

                match status {
                    Ok(exit_status) if exit_status.success() => {
                        println!("Processed audio file saved to: {}", output_file);
                    }
                    Ok(exit_status) => {
                        eprintln!("Error processing audio file {}: runtime exited with status {}", audio_file, exit_status);
                    }
                    Err(e) => {
                        eprintln!("Error running runtime binary: {}", e);
                    }
                }
            } else {
                eprintln!("program file not found: {}", program_path);
            }
        });
    });
}