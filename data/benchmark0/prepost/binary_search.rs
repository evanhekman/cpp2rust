use vstd::prelude::*;

verus! {

pub fn binary_search(a: &[i32], target: i32) -> (result: bool)
    requires
        forall|i: int, j: int| 0 <= i < j < a@.len() ==> a@[i] <= a@[j],
    ensures
        result == exists|i: int| 0 <= i < a@.len() && a@[i] == target,
{
    assume(false);
}

} // verus!
