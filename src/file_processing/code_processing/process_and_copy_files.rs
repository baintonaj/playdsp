use crate::constants::constants::*;
use std::fs::copy;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub(crate) fn process_and_copy_files(folder_path: &str, file_type: &str) -> io::Result<()> {
    let files = get_files_from_folder(folder_path)?;
    if files.is_empty() {
        eprintln!(
            "No rust_process_audio.rs or cpp_process_audio.cpp found in '{}'",
            folder_path
        );
    }

    for file in files {
        let file_path = match file.to_str() {
            Some(p) => p,
            None => {
                eprintln!("Skipping file with non-UTF-8 path");
                continue;
            }
        };
        let file_name = match Path::new(file_path).file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => {
                eprintln!("Skipping file with invalid name: {}", file_path);
                continue;
            }
        };

        if (file_type == "rust" && file_name == "rust_process_audio.rs")
            || (file_type == "cpp" && file_name == "cpp_process_audio.cpp")
            || (file_type == "both"
                && (file_name == "rust_process_audio.rs" || file_name == "cpp_process_audio.cpp"))
        {
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
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if file_name == "cpp_process_audio.cpp" || file_name == "rust_process_audio.rs" {
            valid_files.push(path);
        }
    }

    Ok(valid_files)
}

fn validate_file(file_path: &str) -> io::Result<bool> {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if file_name == "cpp_process_audio.cpp" {
        return check_cpp_function_signature(file_path);
    } else if file_name == "rust_process_audio.rs" {
        return check_rust_function_signature(file_path);
    }

    Ok(false)
}

const CPP_SIGNATURE: &str =
    "extern \"C\" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output)";
const RUST_SIGNATURE: &str =
    "pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>)";

fn check_cpp_function_signature(file_path: &str) -> io::Result<bool> {
    Ok(contains_signature(&read_file(file_path)?, CPP_SIGNATURE))
}

fn check_rust_function_signature(file_path: &str) -> io::Result<bool> {
    Ok(contains_signature(&read_file(file_path)?, RUST_SIGNATURE))
}

fn read_file(file_path: &str) -> io::Result<String> {
    let mut contents = String::new();
    fs::File::open(file_path)?.read_to_string(&mut contents)?;
    Ok(contents)
}

// Whitespace-insensitive match: both sides have all whitespace removed, so
// `const double* input`, `const double *input` and line-wrapped parameter
// lists all match. `externC` cannot occur in valid code, so collapsing the
// space between keywords doesn't create false positives in practice.
fn contains_signature(contents: &str, signature: &str) -> bool {
    let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    strip(contents).contains(&strip(signature))
}

fn copy_to_processing_folder(file_path: &str) -> io::Result<()> {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Could not extract filename from path '{}'", file_path),
            )
        })?;

    let destination = if file_name == "rust_process_audio.rs" {
        RUST_FOLDER.join(file_name)
    } else if file_name == "cpp_process_audio.cpp" {
        CPP_FOLDER.join(file_name)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unknown file type",
        ));
    };

    if file_name == "rust_process_audio.rs" {
        fs::create_dir_all(&*RUST_FOLDER)?;
    } else {
        fs::create_dir_all(&*CPP_FOLDER)?;
    }

    copy(file_path, &destination)?;
    println!("File copied to: {}", destination.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_conventionally_formatted_signatures() {
        let rust = "pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {";
        assert!(contains_signature(rust, RUST_SIGNATURE));
        let cpp = "extern \"C\" void cpp_process(const double* input, size_t num_channels,\n                             size_t num_samples, double* output) {";
        assert!(contains_signature(cpp, CPP_SIGNATURE));
    }

    #[test]
    fn accepts_unusual_whitespace() {
        let rust = "pub  fn rust_process (\n    input : & Vec<Vec<f64>>,\n    output: &mut Vec< Vec<f64> >\n)";
        assert!(contains_signature(rust, RUST_SIGNATURE));
        let cpp = "extern \"C\"\nvoid cpp_process(const double *input, size_t num_channels, size_t num_samples, double *output)";
        assert!(contains_signature(cpp, CPP_SIGNATURE));
    }

    #[test]
    fn rejects_wrong_signatures() {
        assert!(!contains_signature("pub fn rust_process(input: &[f64], output: &mut [f64])", RUST_SIGNATURE));
        assert!(!contains_signature("fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>)", RUST_SIGNATURE));
        assert!(!contains_signature("void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output)", CPP_SIGNATURE));
        assert!(!contains_signature("extern \"C\" void cpp_process(const float* input, size_t num_channels, size_t num_samples, float* output)", CPP_SIGNATURE));
    }
}
