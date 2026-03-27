// depth: 7
int sum_skip_first(int* a, int n) {
    int s = 0;
    for (int i = 1; i < n; i++) {
        s += a[i];
    }
    return s;
}
