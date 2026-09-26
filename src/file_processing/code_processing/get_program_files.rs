use std::fs;

pub(crate) fn get_program_files(folder: &str, extension: &str) -> Vec<String> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path
                    .extension()
                    .map(|ext| ext == extension)
                    .unwrap_or(false)
                {
                    if let Some(path_str) = path.to_str() {
                        if path_str.contains("process_audio") {
                            files.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }

    files
}

// True if the folder (recursively) contains any C++ source or header files.
pub(crate) fn has_cpp_files(dir: &std::path::Path) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if has_cpp_files(&path) {
                    return true;
                }
            } else {
                let ext = path.extension().and_then(|s| s.to_str());
                if ext == Some("cpp") || ext == Some("h") || ext == Some("hpp") {
                    return true;
                }
            }
        }
    }
    false
}
