use crate::constants::constants::*;
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::Path;
use std::process::Command;

pub(crate) fn process_multiple_audio_files(audio_files: &[String], program_paths: &[String], preserve_meta: bool) {
    let runtime_binary = std::path::PathBuf::from("../audio/.playdsp_runtime/target/release")
        .join(format!("playdsp_runtime{}", std::env::consts::EXE_SUFFIX));

    if !runtime_binary.exists() {
        eprintln!("Runtime binary not found. This shouldn't happen after recompilation.");
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&*RESULT_FOLDER) {
        eprintln!("Could not create result folder {}: {}", RESULT_FOLDER.display(), e);
        return;
    }

    let pairs: Vec<(&String, &String)> = audio_files
        .iter()
        .flat_map(|audio| program_paths.iter().map(move |prog| (audio, prog)))
        .collect();

    let pb = ProgressBar::new(pairs.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let processing_start = std::time::Instant::now();

    pairs.par_iter().for_each(|(audio_file, program_path)| {
        let current_time = Local::now().format("%Y_%m_%d_%H_%M_%S_%3f").to_string();
        let audio_stem = Path::new(audio_file.as_str())
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let program_suffix = Path::new(program_path.as_str())
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        if Path::new(program_path.as_str()).exists() {
            let output_file = RESULT_FOLDER.join(format!(
                "{}_processed_{}_{}.wav",
                audio_stem, current_time, program_suffix
            ));

            let mut cmd = Command::new(&runtime_binary);
            cmd.arg(audio_file.as_str())
                .arg(&output_file)
                .arg(program_suffix);
            if preserve_meta {
                cmd.arg("--meta");
            }

            // Capture the child's output so its warnings are printed above the
            // progress bar instead of being drawn over it.
            match cmd.output() {
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    for line in stderr.lines().filter(|l| !l.trim().is_empty()) {
                        pb.println(format!("    {}", line));
                    }
                    if output.status.success() {
                        pb.println(format!("  → {}", output_file.display()));
                    } else {
                        pb.println(format!(
                            "  ✗ {}: runtime exited with {}",
                            audio_file, output.status
                        ));
                    }
                }
                Err(e) => {
                    pb.println(format!("  ✗ runtime error: {}", e));
                }
            }
        } else {
            pb.println(format!("  ✗ program file not found: {}", program_path));
        }

        pb.inc(1);
    });

    pb.finish_with_message("done");
    println!(
        "All files processed in {:.1}s",
        processing_start.elapsed().as_secs_f64()
    );
}
