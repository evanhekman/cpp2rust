// depth: 9
int count_in_range(int* a, int n, int lo, int hi) {
    int c = 0;
    for (int i = 0; i < n; i++) {
        if (a[i] >= lo && a[i] <= hi) c += 1;
    }
    return c;
}
