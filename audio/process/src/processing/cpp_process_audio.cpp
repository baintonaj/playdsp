#include <cstddef>
#include <cmath>

extern "C" void cpp_process_audio(const double* input, double* output, std::size_t num_samples, std::size_t num_channels) {
    double gain_raw = std::pow(10.0, 0.0 / 20.0);

    for (std::size_t channel = 0; channel < num_channels; ++channel) {
        for (std::size_t sample = 0; sample < num_samples; ++sample) {
            std::size_t idx = channel * num_samples + sample;
            output[idx] = input[idx] * gain_raw;
        }
    }
}