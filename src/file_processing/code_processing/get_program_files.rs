use std::fs;

pub(crate) fn get_program_files(folder: &str, extension: &str) -> Vec<String> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map(|ext| ext == extension).unwrap_or(false) {
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