#include <cstddef>
#include <cmath>

extern "C" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output) {
    double gain_raw = std::pow(10.0, -12.0 / 20.0);
    std::size_t total_samples = num_channels * num_samples;

    for (std::size_t i = 0; i < total_samples; ++i) {
        output[i] = input[i] * gain_raw;
    }
}