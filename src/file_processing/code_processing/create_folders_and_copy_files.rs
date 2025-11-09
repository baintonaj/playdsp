use std::fs::*;
use std::path::Path;
use crate::constants::constants::*;

pub(crate) fn create_folders_and_copy_files(base_dir: &str) {
    let audio_dir = Path::new(base_dir).join(AUDIO_NAME);
    let processing_dir = audio_dir.join(PROCESSING_NAME);
    let rust_dir = processing_dir.join("rust");
    let cpp_dir = processing_dir.join("cpp");
    let result_dir = audio_dir.join(RESULT_NAME);
    let source_dir = audio_dir.join(SOURCE_NAME);

    create_dir_all(&audio_dir).expect(&*("Failed to create '".to_owned() + AUDIO_NAME + "' parent directory"));
    create_dir_all(&processing_dir).expect(&*("Failed to create '".to_owned() + PROCESSING_NAME + "' directory"));
    create_dir_all(&rust_dir).expect("Failed to create 'rust' directory");
    create_dir_all(&cpp_dir).expect("Failed to create 'cpp' directory");
    create_dir_all(&result_dir).expect(&*("Failed to create '".to_owned() + RESULT_NAME + "' directory"));
    create_dir_all(&source_dir).expect(&*("Failed to create '".to_owned() + SOURCE_NAME + "' directory"));

    let rust_file_content =
r#"pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let gain_db = -12.0;
    let gain_linear = 10.0_f64.powf(gain_db / 20.0);

    for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
        for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
            *out_sample = in_sample * gain_linear;
        }
    }
}"#;

    let cpp_file_content =
r#"#include <cstddef>
#include <cmath>
#include <vector>

extern "C" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output) {
    std::vector<std::vector<double>> input_vector(num_channels, std::vector<double>(num_samples, 0.0));
    std::vector<std::vector<double>> output_vector(num_channels, std::vector<double>(num_samples, 0.0));

    std::size_t k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++) {
        for (std::size_t channel = 0; channel < num_channels; channel++) {
            input_vector[channel][sample] = input[k];
            k++;
        }
    }

    double gain_db = -12.0;
    double gain_linear = std::pow(10.0, gain_db / 20.0);

    for (std::size_t channel = 0; channel < num_channels; channel++) {
        for (std::size_t sample = 0; sample < num_samples; sample++) {
            output_vector[channel][sample] = input_vector[channel][sample] * gain_linear;
        }
    }

    k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++) {
        for (std::size_t channel = 0; channel < num_channels; channel++) {
            output[k] = output_vector[channel][sample];
            k++;
        }
    }
}"#;

    let rust_file_path = rust_dir.join("rust_process_audio.rs");
    let cpp_file_path = cpp_dir.join("cpp_process_audio.cpp");

    write(&rust_file_path, rust_file_content).expect("Failed to write Rust file");
    write(&cpp_file_path, cpp_file_content).expect("Failed to write C++ file");

    println!("Created folder structure with rust/ and cpp/ subdirectories");
    println!("Rust processing files: {}", rust_dir.display());
    println!("C++ processing files: {}", cpp_dir.display());
    println!("Place your audio files in: {}", source_dir.display());
}