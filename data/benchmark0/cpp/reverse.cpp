/*
reverse an array in-place
- the c++ implementation uses two raw pointers advancing toward each other from both ends
- a naive rust implementation would keep pointer arithmetic with unsafe
- a good rust implementation uses two indices on a &mut [i32] slice
*/

void reverse(int* a, int n) {
    int* lo = a;
    int* hi = a + n - 1;
    while (lo < hi) {
        int tmp = *lo;
        *lo = *hi;
        *hi = tmp;
        lo++;
        hi--;
    }
}
