
use vstd::prelude::*;

verus! {

#[verifier::loop_isolation(false)]

// Pre:  a is non-empty
// Post: a is the reverse of its original contents
pub fn reverse(a: &mut [i32])
    requires
        old(a)@.len() >= 1,
    ensures
        a@.len() == old(a)@.len(),
        forall|k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[a@.len() - 1 - k],
{
    if a.len() == 0 { return; }
    let mut lo = 0usize;
    let mut hi = a.len() - 1;
    while lo < hi
        invariant
            old(a)@.len() >= 1,
            a@.len() == old(a)@.len(),
            // The loop modifies a through a.swap(lo, hi), so the existing invariant for a is left as is.
            forall |k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[k] || (k == lo || k == hi) ==> a@[k] == old(a)@[if k == lo { ( hi ) as int } else { ( lo ) as int }]
    {
        a.swap(lo, hi);
        lo += 1;
        hi -= 1;
    }
}

} // verus!


// Score: (0, 1)
// Safe: True