// depth: 7
int sum_backwards(int* a, int n) {
    int s = 0;
    for (int i = n-1; i >= 0; i--) {
        s += a[i];
    }
    return s;
}
