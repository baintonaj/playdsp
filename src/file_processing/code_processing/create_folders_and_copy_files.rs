use std::fs::*;
use std::path::Path;
use crate::constants::constants::*;

pub(crate) fn create_folders_and_copy_files(base_dir: &str) {
    let audio_dir = Path::new(base_dir).join(AUDIO_NAME);
    let processing_dir = audio_dir.join(PROCESSING_NAME);
    let result_dir = audio_dir.join(RESULT_NAME);
    let source_dir = audio_dir.join(SOURCE_NAME);

    create_dir_all(&audio_dir).expect(&*("Failed to create '".to_owned() + AUDIO_NAME + "' parent directory"));
    create_dir_all(&processing_dir).expect(&*("Failed to create '".to_owned() + PROCESSING_NAME + "' directory"));
    create_dir_all(&result_dir).expect(&*("Failed to create '".to_owned() + RESULT_NAME + "' directory"));
    create_dir_all(&source_dir).expect(&*("Failed to create '".to_owned() + SOURCE_NAME + "' directory"));

    let rust_file_content =
        r#"pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let gain_raw = 10.0_f64.powf(-12.0 / 20.0);

    for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
        for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
            *out_sample = in_sample * gain_raw;
        }
    }
}"#;

    let cpp_file_content =
        r#"#include <cstddef>
#include <cmath>

extern "C" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output) {
    double gain_raw = std::pow(10.0, -12.0 / 20.0);
    std::size_t total_samples = num_channels * num_samples;

    for (std::size_t i = 0; i < total_samples; ++i) {
        output[i] = input[i] * gain_raw;
    }
}"#;

    let rust_file_path = processing_dir.join("rust_process_audio.rs");
    let cpp_file_path = processing_dir.join("cpp_process_audio.cpp");

    // Write Rust and C++ processing files in the "processing" folder
    write(&rust_file_path, rust_file_content).expect("Failed to write Rust file");
    write(&cpp_file_path, cpp_file_content).expect("Failed to write C++ file");

    println!("Created folder structure and placed processing files in {}", audio_dir.display());
}