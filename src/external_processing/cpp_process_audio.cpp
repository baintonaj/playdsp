#include <cstddef>
#include <cmath>
#include <vector>

extern "C" void cpp_process(const double* input, size_t num_channels, size_t num_samples, double* output) {
    std::vector<std::vector<double>/* */> input_vector(num_channels, std::vector<double>(num_samples, 0.0));
    std::vector<std::vector<double>/* */> output_vector(num_channels, std::vector<double>(num_samples, 0.0));

    // Expand to 2D
    std::size_t k_in = 0;
    for (std::size_t i = 0; i < num_channels; i++) {
        for (std::size_t j = 0; j < num_samples; j++) {
            input_vector[i][j] = input[k_in];
            k_in++;
        }
    }

    // Core Processing
    double gain_raw = std::pow(10.0, -12.0 / 20.0);
    for (std::size_t i = 0; i < num_channels; i++) {
        for (std::size_t j = 0; j < num_samples; j++) {
            output_vector[i][j] = input_vector[i][j] * gain_raw;
        }
    }

    // Flatten to 1D
    std::size_t k_out = 0;
    for (std::size_t i = 0; i < num_channels; i++) {
        for (std::size_t j = 0; j < num_samples; j++) {
            output[k_out] = output_vector[i][j];
            k_out++;
        }
    }
}