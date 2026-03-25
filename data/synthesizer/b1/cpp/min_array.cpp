// depth: 8
int min_array(int* a, int n) {
    int m = a[0];
    for (int i = 0; i < n; i++) {
        if (a[i] < m) m = a[i];
    }
    return m;
}
