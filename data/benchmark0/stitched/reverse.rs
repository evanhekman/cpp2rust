use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut [i32])
    requires
        old(a)@.len() >= 1,
    ensures
        a@.len() == old(a)@.len(),
        forall|k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[a@.len() - 1 - k],
{
    let mut lo: usize = 0usize; let mut hi: usize = (a.len() - 1usize); while (lo < hi) { let mut tmp: i32 = a[lo]; a[lo] = a[hi]; a[hi] = tmp; lo += 1usize; hi -= 1usize; } 
}

} // verus!
