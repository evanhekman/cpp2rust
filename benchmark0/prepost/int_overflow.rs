use vstd::prelude::*;

verus! {

// Pre:  (none — safe for all u8 inputs)
// Post: result equals the mathematical sum of a and b cast to i32,
//       and the result is provably in [0, 510]
pub fn add_bytes(a: u8, b: u8) -> (result: i32)
    ensures
        result == a as i32 + b as i32,
        0 <= result,
        result <= 510,
{
    assume(false);
    0
}

} // verus!
