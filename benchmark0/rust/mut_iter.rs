// Index-based loop with Vec::remove avoids iterator invalidation entirely.
// The borrow checker statically rules out the C++ UB at compile time.
// Note: retain(|x| x % 2 != 0) is more idiomatic but lacks a Verus spec.

pub fn remove_evens(v: &mut Vec<i32>) {
    let mut i = 0;
    while i < v.len() {
        if v[i] % 2 == 0 {
            v.remove(i);
        } else {
            i += 1;
        }
    }
}
