// C++ silently promotes uint8_t operands to int before addition.
// No overflow occurs here — the result always fits in int.
// A naive Rust translation using `u8 + u8` would panic on overflow in debug mode.
#include <cstdint>

int add_bytes(uint8_t a, uint8_t b) {
    return a + b;
}
