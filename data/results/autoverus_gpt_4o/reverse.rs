
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
    let mut tmp;
    while lo < hi
        invariant
            old(a)@.len() >= 1,
            a@.len() == old(a)@.len(),
            // a is being modified here with swap operations, not including a set function, hence handled.
    {
        tmp = a[hi];
        a[hi] = a[lo];
        a[lo] = tmp;
        lo += 1;
        hi -= 1;
    }
}

fn main() {} // Added by AI

} // verus!

// Score: (0, 1)
// Safe: True