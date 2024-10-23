mod processing;
pub use processing::rust_process_audio::*;

extern "C" {
    fn cpp_process_audio(buffered_samples: *const f64, processed_samples: *mut f64, num_samples: usize, num_channels: usize);
}

pub fn cpp_process(input: &Vec<Vec<f64>>, output: &mut Vec<Vec<f64>>) {
    let num_samples = input[0].len();
    let num_channels = input.len();

    let input_flat: Vec<f64> = input.iter().flat_map(|channel| channel.iter().cloned()).collect();
    let mut output_flat: Vec<f64> = vec![0.0; num_samples * num_channels];

    unsafe {
        cpp_process_audio(input_flat.as_ptr(), output_flat.as_mut_ptr(), num_samples, num_channels);
    }

    for (channel_idx, channel) in output.iter_mut().enumerate() {
        for sample_idx in 0..num_samples {
            channel[sample_idx] = output_flat[channel_idx * num_samples + sample_idx];
        }
    }
}