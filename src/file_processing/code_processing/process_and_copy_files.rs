use std::{fs, io};
use std::fs::copy;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::constants::constants::*;

pub(crate) fn process_and_copy_files(folder_path: &str, file_type: &str) -> io::Result<()> {
    let files = get_files_from_folder(folder_path)?;

    for file in files {
        let file_path = file.to_str().unwrap();
        let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();

        if (file_type == "rust" && file_name == "rust_process_audio.rs") ||
            (file_type == "cpp" && file_name == "cpp_process_audio.cpp") ||
            (file_type == "both" && (file_name == "rust_process_audio.rs" || file_name == "cpp_process_audio.cpp")) {

            if validate_file(file_path)? {
                copy_to_processing_folder(file_path)?;
            } else {
                eprintln!("Invalid file or function signature: {}", file_path);
            }
        }
    }

    Ok(())
}

fn get_files_from_folder(folder_path: &str) -> io::Result<Vec<PathBuf>> {
    let mut valid_files = vec![];
    let entries = fs::read_dir(folder_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap_or("");

        if file_name == "cpp_process_audio.cpp" || file_name == "rust_process_audio.rs" {
            valid_files.push(path);
        }
    }

    Ok(valid_files)
}

fn validate_file(file_path: &str) -> io::Result<bool> {
    let file_name = Path::new(file_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap_or("");

    if file_name == "cpp_process_audio.cpp" {
        return check_cpp_function_signature(file_path);
    } else if file_name == "rust_process_audio.rs" {
        return check_rust_function_signature(file_path);
    }

    Ok(false)
}

fn check_cpp_function_signature(file_path: &str) -> io::Result<bool> {
    let mut file = fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents.contains("extern \"C\" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output)"))
}

fn check_rust_function_signature(file_path: &str) -> io::Result<bool> {
    let mut file = fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents.contains("pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>)"))
}

fn copy_to_processing_folder(file_path: &str) -> io::Result<()> {
    let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();
    let destination = format!("{}/{}", PROGRAM_FOLDER, file_name);

    copy(file_path, &destination)?;
    println!("File copied to: {}", destination);

    Ok(())
}