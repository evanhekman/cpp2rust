use vstd::prelude::*;
fn main() {}
verus! {

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    ensures
        result == exists|i: int| 0 <= i && i < a@.len() && a@[i] as i32 == target,
{
    for i in 0..a.len()
        invariant
            0 <= i as int,
            i as int <= a@.len(),
            forall|j: int| 0 <= j && j < i as int ==> a@[j] as i32 != target,
    {
        if (target == (a[i] as i32)) { return true; }
    }
    return false;
}

} // verus!

// Score: (2, 0)
// Safe: True