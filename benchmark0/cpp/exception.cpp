// Two preprocessing challenges:
//   1. pointer+length params (int* v, int n) → &[i32] slice, n is dropped
//   2. exception used as early-exit control flow, not error handling
// A naive translation both keeps the n parameter and wraps the return in Result<>.

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
