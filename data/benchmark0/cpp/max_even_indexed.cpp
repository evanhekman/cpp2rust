/*
finding the maximum even-indexed element in a list
- the c++ implementation uses pointer arithmetic and requires a size param
- the naive rust implementation keeps the dummy size parameter and has to use unsafe for (p += 2)
- the good rust implementation drops the dummy size parameter and indexes to avoid pointer issues
*/

int max_even_indexed(int* a, int n) {
    int m = a[0];
    for (int* p = a + 2; p < a + n; p += 2)
        if (*p > m) m = *p;
    return m;
}
