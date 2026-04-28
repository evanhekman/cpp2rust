use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut [i32]) -> (result: ())
    requires

    ensures
        a@ == old(a)@.reverse(),
{
    let mut lo: usize = 0usize; let mut hi: usize = (a.len() - 1usize); while (lo < hi) { let mut tmp: i32 = a[lo]; a[lo] = a[hi]; a[hi] = tmp; lo += 1usize; hi -= 1usize; } 
}

} // verus!
