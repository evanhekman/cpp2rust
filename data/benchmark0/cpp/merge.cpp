#include <cstdint>

void merge(int* a, int m, int* b, int n, int* out) {
    int i = 0, j = 0, k = 0;
    while (i < m && j < n) {
        if (a[i] <= b[j]) out[k++] = a[i++];
        else out[k++] = b[j++];
    }
    while (i < m) out[k++] = a[i++];
    while (j < n) out[k++] = b[j++];
}
