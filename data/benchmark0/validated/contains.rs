use vstd::prelude::*;

verus! {

#[verifier::loop_isolation(false)]

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    ensures
        result == exists|i: int| 0 <= i && i < a@.len() && a@[i] as i32 == target,
{
    for i in 0..a.len() { if (target == (a[i] as i32)) { return true; } } return false; 
}

} // verus!
// Score: (0, 1)
// Safe: None