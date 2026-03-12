// Raw pointer + explicit length: the C idiom for passing arrays.
// Uses an int accumulator which silently overflows for large inputs.
// A naive Rust translation requires unsafe pointer indexing.
#include <cstdint>

int dot(uint8_t* a, uint8_t* b, int n) {
    int sum = 0;
    for (int i = 0; i < n; i++) {
        sum += a[i] * b[i];
    }
    return sum;
}
