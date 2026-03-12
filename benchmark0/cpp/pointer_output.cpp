// C++ output pointer idiom: caller allocates the result location.
// *a + *b has undefined behavior if the sum overflows int.
// A naive Rust translation requires *mut i32 and unsafe dereferences.

void add(int* a, int* b, int* out) {
    *out = *a + *b;
}
