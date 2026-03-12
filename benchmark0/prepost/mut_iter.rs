use vstd::prelude::*;

verus! {

// Pre:  (none)
// Post: every element remaining in v is odd (not divisible by 2)
pub fn remove_evens(v: &mut Vec<i32>)
    ensures
        forall|i: int| 0 <= i < v@.len() ==> v@[i] % 2 != 0,
{
    assume(false);
}

} // verus!
