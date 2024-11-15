use crate::constants::constants::*;
use clap::ArgMatches;
use std::fs::copy;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::process::Command;
use std::{fs, io, process};

fn get_documents_playdsp_path() -> Option<PathBuf> {
    let home_dir = home::home_dir()?;
    Some(home_dir.join("Documents").join("playdsp"))
}

pub(crate) fn run_recompile(matches: &ArgMatches) {
    let user_path =  get_documents_playdsp_path().unwrap().to_str().unwrap().to_owned();

    let playdsp_path = &(user_path.clone() + SRC_DIR);

    if let Err(e) = copy_processing_files(user_path.clone()) {
        eprintln!("Failed to copy processing files: {}", e);
        exit(1);
    }

    println!("Processing files copied successfully.");

    println!("Recompiling code...");
    // Compile and install using sudo
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(user_path.clone())
        .status()
        .expect("Failed to run cargo build");

    if !status.success() {
        eprintln!("Failed to recompile");
        exit(1);
    } else {
        println!("Recompilation complete.");
    }

    // Run the chmod command
    let result = Command::new("chmod")
        .arg("+x")
        .arg(playdsp_path)
        .status();

    match result {
        Ok(status) if status.success() => {
            println!("playdsp made executable successfully.");
        }
        Ok(status) => {
            eprintln!("Command failed with status: {}", status);
        }
        Err(err) => {
            eprintln!("Failed to execute command: {}", err);
        }
    }

    let mut args = vec![];

    if matches.contains_id("rust") {
        args.push("--rust");
    }
    if matches.contains_id("cpp") {
        args.push("--cpp");
    }
    if let Some(audio_path) = matches.get_one::<String>(AUDIO_NAME) {
        args.push("--audio");
        args.push(audio_path);
    }

    let result = Command::new(playdsp_path)
        .args(&args)
        .spawn();

    match result {
        Ok(mut child) => {
            if let Err(e) = child.wait() {
                eprintln!("playdsp process failed to wait: {}", e);
            } else {
                println!("playdsp finished executing.");
                // Optionally exit the current Rust process if needed
            }
        }
        Err(err) => {
            eprintln!("Failed to run playdsp: {}", err);
        }
    }

    if matches.contains_id("code") {
        println!("Exiting current process: PID {}", process::id());
        exit(0);
    }
}

fn copy_processing_files(user_path: String) -> io::Result<()> {
    let source_dir = Path::new(PROGRAM_FOLDER);

    let binding = user_path + DESTINATION_DIR;
    let destination_dir = Path::new(&binding);

    if !destination_dir.exists() {
        fs::create_dir_all(destination_dir)?;
    }

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            let file_name = entry.file_name();
            let destination_path = destination_dir.join(&file_name);
            let destination_path_clone = destination_path.clone();
            copy(entry.path(), destination_path)?;
            println!("Copied {:?} to {:?}", entry.path(), destination_path_clone);
        }
    }

    Ok(())
}