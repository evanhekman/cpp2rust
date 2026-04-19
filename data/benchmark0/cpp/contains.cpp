/*
linear search: does the byte array contain a target value?
- the c++ implementation uses pointer + length with early return
- a naive rust implementation would keep the pointer arithmetic and return mid-loop
- a good rust implementation uses &[u8] and .iter().any()
- the synthesizer must discover (a[i] as i32) == target to bridge the u8/int type mismatch

note: target is declared before n so the harness passes args as (a, target, n)
*/

#include <cstdint>

bool contains(uint8_t* a, int target, int n) {
    for (int i = 0; i < n; i++)
        if (a[i] == target) return true;
    return false;
}
