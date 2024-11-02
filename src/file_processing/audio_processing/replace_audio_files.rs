use std::{fs, io};
use std::fs::{copy, remove_file};
use std::io::Error;
use std::path::Path;
use crate::constants::constants::*;
use crate::file_processing::audio_processing::get_audio_files_from_folder::*;

pub(crate) fn replace_audio_files(input_folder: &str) -> io::Result<()> {
    let input_wav_files = get_audio_files_from_folder(input_folder);
    let input_wav_files_len = input_wav_files.len();

    if input_wav_files.is_empty() {
        return Err(Error::new(io::ErrorKind::NotFound, "No valid .wav files found in the replace folder"));
    }

    let source_entries = fs::read_dir(SOURCE_FOLDER)?;
    for entry in source_entries {
        let path = entry?.path();
        if path.is_file() {
            remove_file(&path)?;
        }
    }

    for input_file in input_wav_files {
        let input_path = Path::new(&input_file);
        let file_name = input_path.file_name().unwrap();

        let destination_path = Path::new(SOURCE_FOLDER).join(file_name);
        copy(input_path, destination_path)?;
    }

    if input_wav_files_len == 1 {
        println!("Valid .wav file '{}' has been copied to '{}'.", input_folder, SOURCE_FOLDER);
    } else {
        println!("All valid .wav files from '{}' have been copied to '{}'.", input_folder, SOURCE_FOLDER);
    }

    Ok(())
}

