/*
dot product of two vector pointers
- the c++ implementation indexes into pointers for the range 0..n
- the naive rust implementation keeps `int n` parameter even though it is strictly unnecessary
- the good rust implementation drops `int n` and uses &[u8] slices, which encode length directly as a fat pointer (data + length)

notes
- the return type changes from `int` to `u32`: c++ silently overflows, rust makes overflow safety explicit via the type
*/

#include <cstdint>

int dot(uint8_t* a, uint8_t* b, int n) {
    int sum = 0;
    for (int i = 0; i < n; i++) {
        sum += a[i] * b[i];
    }
    return sum;
}
