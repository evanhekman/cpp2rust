// Early return replaces throw/catch — no Result<> wrapper needed.
// Returns the index of the first negative element, or -1 if none exists.
// The slice type encodes the length; the explicit `n` parameter disappears.

pub fn first_negative(v: &[i32]) -> i32 {
    for i in 0..v.len() {
        if v[i] < 0 {
            return i as i32;
        }
    }
    -1
}
