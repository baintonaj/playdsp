use std::fs;
use std::path::Path;

pub(crate) fn get_audio_files_from_folder(source: &str) -> Vec<String> {
    let path = Path::new(source);
    if path.is_dir() {
        match fs::read_dir(source) {
            Ok(entries) => entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.extension().map(|ext| ext == "wav").unwrap_or(false) {
                        path.to_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            Err(e) => {
                eprintln!("Error reading source directory '{}': {}", source, e);
                vec![]
            }
        }
    } else {
        vec![source.to_string()]
    }
}
