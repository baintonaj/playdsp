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
r#"// ============================================================================
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
// PERSISTENT STATE — LazyLock<Mutex<T>>
// ---------------------------------------
// Declare a state struct and wrap it in a LazyLock<Mutex<...>> static. The
// static is initialised exactly once (on the first buffer call) and lives
// for the entire duration of the run. Lock it at the top of rust_process()
// to obtain a mutable reference valid for the duration of that call.
//
// The channel count is only known at call time, so size any per-channel
// Vecs lazily (resize on first call, or when the Vec is too short).
//
// Example — a simple 1-sample feedback delay across every channel:
//
//   use std::sync::{LazyLock, Mutex};
//
//   struct State {
//       prev_sample: Vec<f64>, // one value per channel; grown lazily
//   }
//
//   static STATE: LazyLock<Mutex<State>> =
//       LazyLock::new(|| Mutex::new(State { prev_sample: Vec::new() }));
//
//   pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
//       let mut state = STATE.lock().unwrap();
//
//       // Extend if channel count wasn't known at initialisation time.
//       if state.prev_sample.len() < input.len() {
//           state.prev_sample.resize(input.len(), 0.0);
//       }
//
//       for (ch, (in_ch, out_ch)) in
//           input.iter().zip(output.iter_mut()).enumerate()
//       {
//           for (x, y) in in_ch.iter().zip(out_ch.iter_mut()) {
//               // Mix current sample with 50% of the previous output.
//               *y = *x + 0.5 * state.prev_sample[ch];
//               state.prev_sample[ch] = *y;
//           }
//       }
//   }
//
// ============================================================================

pub fn rust_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let gain_db = -12.0;
    let gain_linear = 10.0_f64.powf(gain_db / 20.0);

    for (in_channel, out_channel) in input.iter().zip(output.iter_mut()) {
        for (in_sample, out_sample) in in_channel.iter().zip(out_channel.iter_mut()) {
            *out_sample = in_sample * gain_linear;
        }
    }
}"#;

    let cpp_file_content =
r#"// ============================================================================
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
// PERSISTENT STATE — static struct
// ----------------------------------
// Declare a plain struct that holds your DSP state, then declare it as a
// static local variable inside cpp_process(). C++ guarantees the variable
// is constructed exactly once (on the first call) and lives for the rest of
// the program. Access it directly by name on subsequent calls.
//
// The channel count is only known at call time, so size any per-channel
// vectors lazily (resize on first call, or when the vector is too short).
//
// Example — a simple 1-sample feedback delay across every channel:
//
//   #include <cstddef>
//   #include <vector>
//
//   struct State {
//       std::vector<double> prev_sample; // one value per channel; grown lazily
//   };
//
//   extern "C" void cpp_process(const double* input, size_t num_channels,
//                               size_t num_samples, double* output) {
//       static State state; // constructed once, persists across every call
//
//       // Extend if channel count wasn't known at construction time.
//       if (state.prev_sample.size() < num_channels)
//           state.prev_sample.resize(num_channels, 0.0);
//
//       for (size_t s = 0; s < num_samples; ++s) {
//           for (size_t ch = 0; ch < num_channels; ++ch) {
//               size_t k = s * num_channels + ch;
//               // Mix current sample with 50% of the previous output.
//               double y = input[k] + 0.5 * state.prev_sample[ch];
//               output[k] = y;
//               state.prev_sample[ch] = y;
//           }
//       }
//   }
//
// ============================================================================

#include <cstddef>
#include <cmath>
#include <vector>

extern "C" void cpp_process(const double* input, size_t num_channels,
                             size_t num_samples, double* output) {
    std::vector<std::vector<double>> input_vector(num_channels,
        std::vector<double>(num_samples, 0.0));
    std::vector<std::vector<double>> output_vector(num_channels,
        std::vector<double>(num_samples, 0.0));

    std::size_t k = 0;
    for (std::size_t sample = 0; sample < num_samples; sample++) {
        for (std::size_t channel = 0; channel < num_channels; channel++) {
            input_vector[channel][sample] = input[k];
            k++;
        }
    }

    double gain_db  = -12.0;
    double gain_linear = std::pow(10.0, gain_db / 20.0);

    for (std::size_t channel = 0; channel < num_channels; channel++) {
        for (std::size_t sample = 0; sample < num_samples; sample++) {
            output_vector[channel][sample] =
                input_vector[channel][sample] * gain_linear;
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