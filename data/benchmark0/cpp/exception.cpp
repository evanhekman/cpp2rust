/*
first negative in an array with early exit
- the c++ implementation uses a throw/catch to exit early, and requires a size param alongside the pointer
- a naive rust implementation would wrap this as Result<> even though the exception is internal, and would keep the redundant size param
- a good rust implementation uses an early return and drops the size param in favour of a &[i32] slice
*/

int first_negative(int* v, int n) {
    try {
        for (int i = 0; i < n; i++) {
            if (v[i] < 0) throw i;
        }
        return -1;
    } catch (int i) {
        return i;
    }
}
