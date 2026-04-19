// depth: 9
int sum_in_range(int* a, int n, int lo, int hi) {
    int s = 0;
    for (int i = 0; i < n; i++) {
        if (a[i] >= lo && a[i] <= hi) s += a[i];
    }
    return s;
}
