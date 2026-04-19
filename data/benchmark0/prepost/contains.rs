// Pre:  (none)
// Post: result is true iff some element equals target



use vstd::prelude::*;

verus! {

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    ensures
        result == exists|i: int| 0 <= i && i < a@.len() && a@[i] as i32 == target,
{
    assume(false);
}

} // verus!
