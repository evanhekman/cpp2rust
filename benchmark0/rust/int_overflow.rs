// Cast to i32 before adding, making the integer promotion explicit.
// Unlike C++, Rust does not silently widen u8 to a larger integer type.
// The result is always in [0, 510] so i32 is sufficient and no panic is possible.

pub fn add_bytes(a: u8, b: u8) -> i32 {
    (a as i32) + (b as i32)
}
