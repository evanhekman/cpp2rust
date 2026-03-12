/*
clamp all elements of an array to [lo, hi]
- the c++ implementation uses a raw pointer and size parameter, modifying elements in-place
- a naive rust implementation would use *mut i32 with unsafe for pointer dereferencing and keep the size param
- a good rust implementation uses a &mut [i32] slice, dropping the size parameter
*/

void clamp_all(int* v, int n, int lo, int hi) {
    for (int i = 0; i < n; i++) {
        if (v[i] < lo) v[i] = lo;
        else if (v[i] > hi) v[i] = hi;
    }
}
