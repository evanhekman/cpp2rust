// depth: 7
int count_positive(int* a, int n) {
    int c = 0;
    for (int i = 0; i < n; i++) {
        if (a[i] > 0) c += 1;
    }
    return c;
}
