use std::fs;
use std::path::Path;

// Case-insensitive: field recorders commonly write `.WAV`.
pub(crate) fn is_wav_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("wav"))
            .unwrap_or(false)
}

pub(crate) fn get_audio_files_from_folder(source: &Path) -> Vec<String> {
    if source.is_dir() {
        match fs::read_dir(source) {
            Ok(entries) => {
                let mut files: Vec<String> = entries
                    .filter_map(|entry| {
                        let path = entry.ok()?.path();
                        if is_wav_file(&path) {
                            path.to_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                files.sort();
                files
            }
            Err(e) => {
                eprintln!("Error reading source directory '{}': {}", source.display(), e);
                vec![]
            }
        }
    } else if is_wav_file(source) {
        vec![source.to_string_lossy().into_owned()]
    } else {
        vec![]
    }
}
