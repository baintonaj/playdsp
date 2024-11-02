use std::fs;
use std::path::Path;

pub(crate) fn get_audio_files_from_folder(source: &str) -> Vec<String> {
    let path = Path::new(source);
    if path.is_dir() {
        fs::read_dir(source)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.extension().map(|ext| ext == "wav").unwrap_or(false) {
                    Some(path.to_str().unwrap().to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![source.to_string()]
    }
}