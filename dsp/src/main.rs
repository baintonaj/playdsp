mod constants;
use std::string::String;
use constants::*;
use clap::{Arg, ArgAction, Command};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use bwavfile::{WaveFmt, WaveWriter, WaveReader};
use audio_process;
use chrono::Local;
use std::fs::{copy, create_dir_all, remove_file};
use std::io::{self, Error, Read};

fn main() {

    let matches = Command::new("playdsp")
        .version("0.1.0")
        .author("Andy Bainton <baintonaj@gmail.com>")
        .about("Compiles Rust and/or C++ files in release mode, and processes multiple audio files concurrently")
        .arg(Arg::new("rust")
            .short('r')
            .long("rust")
            .required(false)
            .num_args(0)
            .action(ArgAction::Set)
            .help("Process with Rust code"))
        .arg(Arg::new("cpp")
            .short('c')
            .long("cpp")
            .required(false)
            .num_args(0)
            .action(ArgAction::Set)
            .help("Process with C++ code"))
        .arg(Arg::new(CODE_FILE_PATH_NAME)
            .short('d')
            .long("code")
            .help("Optional folder path containing .cpp or .rs files")
            .required(false)
            .num_args(1)
            .action(ArgAction::Set))
        .arg(Arg::new(AUDIO_FILE_PATH_NAME)
            .short('a')
            .long("audio")
            .help("Optional folder path containing .wav files")
            .required(false)
            .num_args(1)
            .action(ArgAction::Set))
        .get_matches();

    let rust_present = matches.contains_id("rust");
    let cpp_present = matches.contains_id("cpp");

    if let Some(folder_path) = matches.get_one::<String>(CODE_FILE_PATH_NAME) {
        if rust_present && !cpp_present {
            if let Err(e) = process_and_copy_files(folder_path, "rust") {
                eprintln!("Error processing folder for Rust: {}", e);
                return;
            }
        } else if cpp_present && !rust_present {
            if let Err(e) = process_and_copy_files(folder_path, "cpp") {
                eprintln!("Error processing folder for C++: {}", e);
                return;
            }
        } else if !rust_present && !cpp_present {
            if let Err(e) = process_and_copy_files(folder_path, "both") {
                eprintln!("Error processing folder for both Rust and C++: {}", e);
                return;
            }
        }
    }

    if let Some(input_folder) = matches.get_one::<String>(AUDIO_FILE_PATH_NAME) {
        if let Err(e) = replace_audio_files(input_folder) {
            eprintln!("Error replacing audio files: {}", e);
            return;
        }
    }

    if !rust_present && !cpp_present {
        println!("Processing with both Rust and C++ code");
    } else if rust_present {
        println!("Processing with Rust code");
    } else if cpp_present {
        println!("Processing with C++ code");
    }

    let audio_files_to_process = get_audio_files_from_folder(SOURCE_NAME);

    create_dir_all(RESULT_NAME).expect(format!("Failed to create {} directory", RESULT_NAME).as_str());

    let mut rust_files: Vec<String> = vec![];
    let mut cpp_files: Vec<String> = vec![];

    if !rust_present && !cpp_present {
        rust_files = get_program_files(&*PROGRAM_FOLDER, "rs");
        cpp_files = get_program_files(&*PROGRAM_FOLDER, "cpp");
    } else if rust_present {
        rust_files = get_program_files(&*PROGRAM_FOLDER, "rs");
    } else if cpp_present {
        cpp_files = get_program_files(&*PROGRAM_FOLDER, "cpp");
    }

    if !rust_present && !cpp_present {
        println!("Rust files: {:?}", rust_files);
        println!("C++ files: {:?}", cpp_files);
        let mut all_files = Vec::new();
        all_files.append(rust_files.as_mut());
        all_files.append(cpp_files.as_mut());
        process_multiple_audio_files(audio_files_to_process, all_files);
    } else if rust_present {
        println!("Rust files: {:?}", rust_files);
        process_multiple_audio_files(audio_files_to_process, rust_files);
    } else if cpp_present {
        println!("C++ files: {:?}", cpp_files);
        process_multiple_audio_files(audio_files_to_process, cpp_files);
    }
}

fn get_audio_files_from_folder(source: &str) -> Vec<String> {
    let path = Path::new(source);
    if path.is_dir() {
        // If it's a directory, collect all .wav files in the folder
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
        // If it's a file, return it as a single entry in a Vec
        vec![source.to_string()]
    }
}

fn replace_audio_files(input_folder: &str) -> io::Result<()> {
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

fn process_and_copy_files(folder_path: &str, file_type: &str) -> io::Result<()> {
    let files = get_files_from_folder(folder_path)?;

    for file in files {
        let file_path = file.to_str().unwrap();
        let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();

        // Copy based on the provided file_type
        if (file_type == "rust" && file_name == "rust_process_audio.rs") ||
            (file_type == "cpp" && file_name == "cpp_process_audio.cpp") ||
            (file_type == "both" && (file_name == "rust_process_audio.rs" || file_name == "cpp_process_audio.cpp")) {

            // Validate and copy only the matching files
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

        // Get only relevant files (Rust or C++)
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

    Ok(contents.contains("extern \"C\" void cpp_process_audio(const double* input, double* output, std::size_t num_samples, std::size_t num_channels)"))
}

fn check_rust_function_signature(file_path: &str) -> io::Result<bool> {
    let mut file = fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents.contains("pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>)"))
}

fn copy_to_processing_folder(file_path: &str) -> io::Result<()> {
    let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();
    let destination = format!("../audio/process/src/processing/{}", file_name);

    copy(file_path, &destination)?;
    println!("File copied to: {}", destination);

    Ok(())
}

fn get_program_files(folder: &str, extension: &str) -> Vec<String> {
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

fn rust_process_audio(buffer: &Vec<Vec<f64>>, processed_buffer: &mut Vec<Vec<f64>>) {
    audio_process::rust_process(buffer, processed_buffer);
}

fn cpp_process_audio_wrapper(buffer: &Vec<Vec<f64>>, processed_buffer: &mut Vec<Vec<f64>>) {
    audio_process::cpp_process(buffer, processed_buffer);
}

fn process_audio(input_path: &str, output_path: &str, program_file: &str) -> Result<(), String> {
    let file_extension = Path::new(program_file)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let (header, samples): (WaveFmt, Vec<Vec<f32>>) = read_wav(input_path)?;

    let samples_f64: Vec<Vec<f64>> = samples.iter()
        .map(|channel| channel.iter().map(|&sample| sample as f64).collect())
        .collect();

    const BUFFER_SIZE: usize = 2048;
    let num_channels = samples_f64.len();
    let total_samples = samples_f64[0].len();
    let num_buffers = (total_samples + BUFFER_SIZE - 1) / BUFFER_SIZE;

    let mut buffered_samples = vec![vec![vec![0.0; BUFFER_SIZE]; num_channels]; num_buffers];

    for i in 0..total_samples {
        let buffer_index = i / BUFFER_SIZE;
        let sample_index = i % BUFFER_SIZE;
        for channel in 0..num_channels {
            buffered_samples[buffer_index][channel][sample_index] = samples_f64[channel][i];
        }
    }

    let mut processed_samples_f64 = vec![vec![vec![0.0; BUFFER_SIZE]; num_channels]; num_buffers];

    if file_extension == "rs" {
        for (buffer_index, buffer) in buffered_samples.iter().enumerate() {
            rust_process_audio(buffer, &mut processed_samples_f64[buffer_index]);
        }
    } else if file_extension == "cpp" {
        for (buffer_index, buffer) in buffered_samples.iter().enumerate() {
            cpp_process_audio_wrapper(buffer, &mut processed_samples_f64[buffer_index]);
        }
    }

    let mut processed_samples_2d_f64 = vec![vec![0.0; total_samples]; num_channels];
    for buffer_index in 0..num_buffers {
        for channel in 0..num_channels {
            for sample_index in 0..BUFFER_SIZE {
                let flat_index = buffer_index * BUFFER_SIZE + sample_index;
                if flat_index < total_samples {
                    processed_samples_2d_f64[channel][flat_index] = processed_samples_f64[buffer_index][channel][sample_index];
                }
            }
        }
    }

    let processed_samples: Vec<Vec<f32>> = processed_samples_2d_f64.iter()
        .map(|channel| channel.iter().map(|&sample| sample as f32).collect())
        .collect();

    if let Err(err) = write_wav(&output_path, &processed_samples, header) {
        return Err(format!("Error writing WAV file: {}", err));
    }

    Ok(())
}

fn read_wav(input_file_name: &str) -> Result<(WaveFmt, Vec<Vec<f32>>), String> {
    let mut r = WaveReader::open(input_file_name).map_err(|e| format!("Error opening WAV file: {}", e))?;
    let input_format = r.format().map_err(|e| format!("Error reading format: {}", e))?;
    let input_sample_count = r.frame_length().map_err(|e| format!("Error reading frame length: {}", e))? as usize;
    let input_channel_count = input_format.channel_count as usize;
    let mut frame_reader = r.audio_frame_reader().map_err(|e| format!("Error reading audio frames: {}", e))?;
    let mut buffer = input_format.create_frame_buffer::<f32>(input_sample_count * input_channel_count);

    frame_reader.read_frames::<f32>(&mut buffer).map_err(|e| format!("Error reading frames: {}", e))?;

    let mut result = vec![vec![0.0; input_sample_count]; input_channel_count];
    let mut k = 0;
    for i in 0..input_sample_count {
        for j in 0..input_channel_count {
            result[j][i] = buffer[k];
            k += 1;
        }
    }

    Ok((input_format, result))
}

fn write_wav(output_path: &str, processed_samples: &Vec<Vec<f32>>, header: WaveFmt) -> Result<(), String> {
    const BITS_PER_SAMPLE_FOR_F32: u16 = 32;
    let output_format = WaveFmt {
        tag: bwavfile::WAVE_TAG_FLOAT,
        channel_count: header.channel_count,
        sample_rate: header.sample_rate,
        bytes_per_second: (header.channel_count * BITS_PER_SAMPLE_FOR_F32 / 8) as u32 * header.sample_rate,
        block_alignment: header.channel_count * BITS_PER_SAMPLE_FOR_F32 / 8,
        bits_per_sample: BITS_PER_SAMPLE_FOR_F32,
        extended_format: None,
    };

    let values_vec: Vec<f32> = (0..processed_samples[0].len())
        .flat_map(|j| processed_samples.iter().map(move |row| row[j]))
        .collect();

    let w = WaveWriter::create(output_path, output_format).unwrap();
    let mut frame_writer = w.audio_frame_writer().unwrap();

    frame_writer.write_frames::<f32>(values_vec.as_slice()).unwrap();
    frame_writer.end().unwrap();

    Ok(())
}

fn process_multiple_audio_files(audio_files: Vec<String>, program_paths: Vec<String>) {
    audio_files.par_iter().for_each(|audio_file| {
        let current_time = Local::now().format("%Y_%m_%d_%H_%M").to_string();
        let audio_stem = Path::new(audio_file).file_stem().unwrap().to_str().unwrap();

        program_paths.par_iter().for_each(|program_path| {
            let program_suffix  = Path::new(program_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if Path::new(&program_path).exists() {
                let rs_output_file = format!("result/{}_processed_{}_{}.wav", audio_stem, current_time, program_suffix);
                if let Err(e) = process_audio(audio_file, &rs_output_file, &program_path) {
                    eprintln!("Error processing audio file {}: {}", audio_file, e);
                } else {
                    println!("Processed audio file saved to: {}", rs_output_file);
                }
            } else {
                eprintln!("program file not found: {}", program_path);
            }
        });
    });
}