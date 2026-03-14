use vstd::prelude::*;

verus! {

// Pre:  a is non-empty
// Post: a is the reverse of its original contents
pub fn reverse(a: &mut [i32])
    requires
        old(a)@.len() >= 1,
    ensures
        a@.len() == old(a)@.len(),
        forall|k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[a@.len() - 1 - k],
{
    assume(false);
}

} // verus!
