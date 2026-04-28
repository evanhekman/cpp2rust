use vstd::prelude::*;

verus! {

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    ensures
        result == exists|i: int| 0 <= i && i < a@.len() && a@[i] as i32 == target,
{
    for i in 0..a.len()
        invariant
            0 <= i <= a.len(),
            forall|k: int| 0 <= k < i ==> a@[k] as i32 != target,
    { 
        if (a[i] as i32) == target { 
            return true; 
        } 
    } 
    return false; 
}

fn main() {}

} // verus!