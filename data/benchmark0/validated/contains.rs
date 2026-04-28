use vstd::prelude::*;

verus! {

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    requires
        0 <= target && target <= 255,
    ensures
        result <==> exists|i: int| 0 <= i && i < a@.len() && (a@[i] as int) == target,
{
    for i in 0..a.len() { if (target == (a[i] as i32)) { return true; } } return false; 
}

} // verus!
