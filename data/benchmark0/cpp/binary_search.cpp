#include <cstdint>

bool binary_search(int* a, int target, int n) {
    int lo = 0, hi = n - 1;
    while (lo <= hi) {
        int mid = lo + (hi - lo) / 2;
        if (a[mid] == target) return true;
        if (a[mid] < target) lo = mid + 1;
        else hi = mid - 1;
    }
    return false;
}
