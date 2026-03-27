// depth: 8
int sum_abs(int* a, int n) {
    int s = 0;
    for (int i = 0; i < n; i++) {
        s += a[i] >= 0 ? a[i] : (0 - a[i]);
    }
    return s;
}
