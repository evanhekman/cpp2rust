// depth: 9
int max_backwards(int* a, int n) {
    int m = a[0];
    for (int i = n-1; i >= 0; i--) {
        if (a[i] > m) m = a[i];
    }
    return m;
}
