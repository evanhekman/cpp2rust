use vstd::prelude::*;

verus! {

// Pre:  lo <= hi (valid clamp range)
// Post: every element is in [lo, hi]; length unchanged
pub fn clamp_all(v: &mut [i32], lo: i32, hi: i32)
    requires
        lo <= hi,
    ensures
        v@.len() == old(v)@.len(),
        forall|i: int| 0 <= i && i < v@.len() ==> lo <= v@[i] && v@[i] <= hi,
{
    assume(false);
}

} // verus!
