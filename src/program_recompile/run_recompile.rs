use std::{env, fs, io};
use std::fs::copy;
use std::path::{Path, PathBuf};
use std::process::exit;
use clap::ArgMatches;
use std::process::Command;
use crate::constants::constants::*;

fn get_documents_playdsp_path() -> Option<PathBuf> {
    let home_dir = home::home_dir()?;
    Some(home_dir.join("Documents").join("playdsp"))
}

pub(crate) fn run_recompile(matches: &ArgMatches) { // Accept matches as a parameter
    let user_path =  get_documents_playdsp_path().unwrap().to_str().unwrap().to_owned();

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
        .current_dir(user_path.clone()) // Set the working directory
        .status()
        .expect("Failed to run cargo build");

    if !status.success() {
        eprintln!("Failed to recompile");
        exit(1);
    }

    // Copy the new binary
    let status = Command::new("sudo")
        .arg("cp")
        .arg("-p")
        .arg( Path::new(&(user_path.clone() + SRC_DIR)))
        .arg( Path::new(&(user_path.clone() + RESULT_DIR)))
        .status()
        .expect("Failed to copy binary");

    if !status.success() {
        eprintln!("Failed to install the new binary");
        exit(1);
    }

    println!("Recompilation and installation complete.");

    let new_binary_path = user_path.clone() + NEW_BINARY_PATH; // Path to the new binary

    // Get the current directory
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let current_dir_str = current_dir.to_str().expect("Failed to convert path to string");

    // Start building the new terminal command
    let mut new_terminal_command = format!(
        "osascript -e 'tell application \"Terminal\" to do script \"cd {} && {}",
        current_dir_str, new_binary_path
    );

    // Check for Rust and C++ flags and append them to the command if they exist
    if matches.contains_id("rust") {
        new_terminal_command.push_str(" --rust");
    }
    if matches.contains_id("cpp") {
        new_terminal_command.push_str(" --cpp");
    }

    // Check for audio file argument and append it to the command if it exists
    if let Some(audio_path) = matches.get_one::<String>(AUDIO_FILE_PATH_NAME) {
        new_terminal_command.push_str(&format!(" --audio {}\"'", audio_path));
    } else {
        new_terminal_command.push_str("\"'"); // Close the string properly
    }

    // Start a new terminal and run the new process in the current directory
    if let Ok(_) = Command::new("sh").arg("-c").arg(new_terminal_command).spawn() {
        println!("Started new instance of playdsp in a new terminal.");
    } else {
        println!("Failed to open new terminal.");
    }

    // Exit the current process
    println!("Exiting current process.");
    exit(0);
}

fn copy_processing_files(user_path: String) -> io::Result<()> {
    let source_dir = Path::new(PROGRAM_FOLDER);

    let binding = user_path + DESTINATION_DIR;
    let destination_dir = Path::new(&binding);

    // Create the destination directory if it doesn't exist
    if !destination_dir.exists() {
        fs::create_dir_all(destination_dir)?;
    }

    // Iterate over the files in the source directory
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        // Only copy regular files
        if file_type.is_file() {
            let file_name = entry.file_name();
            let destination_path = destination_dir.join(&file_name);
            let destination_path_clone = destination_path.clone();
            // Copy the file to the destination directory, replacing if it exists
            copy(entry.path(), destination_path)?;
            println!("Copied {:?} to {:?}", entry.path(), destination_path_clone);
        }
    }

    Ok(())
}