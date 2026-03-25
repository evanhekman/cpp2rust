// depth: 8
int sum_positive(int* a, int n) {
    int s = 0;
    for (int i = 0; i < n; i++) {
        if (a[i] > 0) s += a[i];
    }
    return s;
}
