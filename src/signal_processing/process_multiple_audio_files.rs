use std::path::Path;
use rayon::prelude::*;
use chrono::Local;
use crate::constants::constants::*;
use crate::external_processing::rust_process_audio::*;
use bwavfile::{WaveFmt, WaveReader, WaveWriter};

pub(crate) fn process_multiple_audio_files(audio_files: &Vec<String>, program_paths: &Vec<String>) {
    audio_files.par_iter().for_each(|audio_file| {
        let current_time = Local::now().format("%Y_%m_%d_%H_%M").to_string();
        let audio_stem = Path::new(audio_file).file_stem().unwrap().to_str().unwrap();

        program_paths.par_iter().for_each(|program_path| {
            let program_suffix  = Path::new(program_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if Path::new(&program_path).exists() {
                let rs_output_file = format!("{}/{}_processed_{}_{}.wav", RESULT_FOLDER, audio_stem, current_time, program_suffix);
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

fn rust_process_audio(buffer: &Vec<Vec<f64>>, processed_buffer: &mut Vec<Vec<f64>>) {
    rust_process(buffer, processed_buffer);
}

extern "C" {
    fn cpp_process(
        input: *const f64,
        num_channels: usize,
        num_samples: usize,
        output: *mut f64,
    );
}

pub fn cpp_process_audio_wrapper(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let num_channels = input.len();
    let num_samples = input[0].len();

    let flattened_input: Vec<f64> = input.iter().flatten().copied().collect();
    let mut flattened_output: Vec<f64> = vec![0.0; num_channels * num_samples];

    unsafe {
        cpp_process(
            flattened_input.as_ptr(),
            num_channels,
            num_samples,
            flattened_output.as_mut_ptr(),
        );
    }

    for (i, channel) in output.iter_mut().enumerate() {
        let start = i * num_samples;
        let end = start + num_samples;
        channel.copy_from_slice(&flattened_output[start..end]);
    }
}