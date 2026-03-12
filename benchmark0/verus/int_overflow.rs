use vstd::prelude::*;

verus! {

// Proof is immediate: Z3 handles the cast arithmetic directly.
// The bound 510 = 255 + 255 follows from u8::MAX + u8::MAX.
pub fn add_bytes(a: u8, b: u8) -> (result: i32)
    ensures
        result == a as i32 + b as i32,
        0 <= result,
        result <= 510,
{
    (a as i32) + (b as i32)
}

} // verus!
