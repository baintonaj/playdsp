use crate::constants::constants::*;
use std::fs::*;
use std::path::Path;

pub(crate) fn create_folders_and_copy_files(base_dir: &str) {
    let audio_dir = Path::new(base_dir).join(AUDIO_NAME);
    let processing_dir = audio_dir.join(PROCESSING_NAME);
    let rust_dir = processing_dir.join("rust");
    let cpp_dir = processing_dir.join("cpp");
    let result_dir = audio_dir.join(RESULT_NAME);
    let source_dir = audio_dir.join(SOURCE_NAME);

    create_dir_all(&audio_dir)
        .expect(&*("Failed to create '".to_owned() + AUDIO_NAME + "' parent directory"));
    create_dir_all(&processing_dir)
        .expect(&*("Failed to create '".to_owned() + PROCESSING_NAME + "' directory"));
    create_dir_all(&rust_dir).expect("Failed to create 'rust' directory");
    create_dir_all(&cpp_dir).expect("Failed to create 'cpp' directory");
    create_dir_all(&result_dir)
        .expect(&*("Failed to create '".to_owned() + RESULT_NAME + "' directory"));
    create_dir_all(&source_dir)
        .expect(&*("Failed to create '".to_owned() + SOURCE_NAME + "' directory"));

    let rust_file_content = r#"// ============================================================================
// rust_process_audio.rs — PlayDSP Rust DSP entry point
// ============================================================================
//
// BUFFER-BY-BUFFER PROCESSING
// ----------------------------
// rust_process() is called once per 1024-sample buffer, sequentially, for
// every buffer in the audio file. Local variables are re-created on every
// call — so filter states, delay-line read/write heads, envelope followers,
// and any other data that must carry over from one buffer to the next must
// live *outside* this function.
//
//
// PERSISTENT STATE — LazyLock<Mutex<State>>
// ------------------------------------------
// All DSP logic lives inside State::process(). Add fields to State for any
// data that must persist across buffer calls (filter registers, delay lines,
// envelope followers, etc.).
//
// The channel count is only known at call time, so size any per-channel
// Vecs lazily — check the length and resize before use:
//
//   if self.prev_sample.len() < input.len() {
//       self.prev_sample.resize(input.len(), 0.0);
//   }
//
// ============================================================================

use std::sync::{LazyLock, Mutex};

struct State {
    // Add per-channel DSP state here.
    // Example: prev_sample: Vec<f64>,
}

impl State {
    fn process(&mut self, input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
        let gain_db = -12.0;
        let gain_linear = 10.0_f64.powf(gain_db / 20.0);

        for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
            for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
                *out_sample = in_sample * gain_linear;
            }
        }
    }
}

static STATE: LazyLock<Mutex<State>> =
    LazyLock::new(|| Mutex::new(State {}));

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    STATE.lock().unwrap().process(input, output);
}"#;

    let cpp_file_content = r#"// ============================================================================
// cpp_process_audio.cpp — PlayDSP C++ DSP entry point
// ============================================================================
//
// BUFFER-BY-BUFFER PROCESSING
// ----------------------------
// cpp_process() is called once per 1024-sample buffer, sequentially, for
// every buffer in the audio file. Local variables are destroyed at the end
// of each call, so filter states, delay-line heads, envelope followers, and
// any data that must carry over between buffers must live outside this
// function.
//
//
// PERSISTENT STATE — State::process() + std::scoped_lock
// -------------------------------------------------------
// All DSP logic lives inside State::process(). Add fields to State for any
// data that must persist across buffer calls (filter registers, delay lines,
// envelope followers, etc.).
//
// cpp_process() also holds static input/output working buffers that are
// resized lazily — avoiding repeated heap allocation on every call.
// All statics are protected by the same std::scoped_lock.
//
// The channel count is only known at call time, so size any per-channel
// vectors lazily — check the size and resize before use:
//
//   if (prev_sample.size() < num_channels)
//       prev_sample.resize(num_channels, 0.0);
//
// ============================================================================

#include <cstddef>
#include <cmath>
#include <mutex>
#include <vector>

struct State {
    // Add per-channel DSP state here.
    // Example: std::vector<double> prev_sample;

    void process(std::vector<std::vector<double>>& input_vector,
                 std::size_t num_channels, std::size_t num_samples,
                 std::vector<std::vector<double>>& output_vector) {
        double gain_db     = -12.0;
        double gain_linear = std::pow(10.0, gain_db / 20.0);

        for (std::size_t channel = 0; channel < num_channels; channel++) {
            for (std::size_t sample = 0; sample < num_samples; sample++) {
                output_vector[channel][sample] =
                    input_vector[channel][sample] * gain_linear;
            }
        }
    }
};

extern "C" void cpp_process(const double* input, size_t num_channels,
                             size_t num_samples, double* output) {
    static std::mutex                        state_mutex;
    static State                             state;
    static std::vector<std::vector<double>>  input_vector;
    static std::vector<std::vector<double>>  output_vector;
    std::scoped_lock lock(state_mutex);

    // Resize working buffers lazily when dimensions change.
    if (input_vector.size() != num_channels ||
        (!input_vector.empty() && input_vector[0].size() != num_samples)) {
        input_vector.assign(num_channels,  std::vector<double>(num_samples, 0.0));
        output_vector.assign(num_channels, std::vector<double>(num_samples, 0.0));
    }

    // Deinterleave: interleaved → [channel][sample].
    std::size_t k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++)
        for (std::size_t channel = 0; channel < num_channels; channel++)
            input_vector[channel][sample] = input[k++];

    state.process(input_vector, num_channels, num_samples, output_vector);

    // Re-interleave: [channel][sample] → interleaved.
    k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++)
        for (std::size_t channel = 0; channel < num_channels; channel++)
            output[k++] = output_vector[channel][sample];
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
