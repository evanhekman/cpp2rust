use vstd::prelude::*;

verus! {

#[verifier::loop_isolation(false)]

pub fn reverse(a: &mut [i32]) -> (result: ())
    requires

    ensures
        a@ == old(a)@.reversed(),
{
    let mut lo: usize = 0usize; let mut hi: usize = (a.len() - 1usize); while (lo < hi) { let mut tmp: i32 = a[lo]; a[lo] = a[hi]; a[hi] = tmp; lo += 1usize; hi -= 1usize; } 
}

} // verus!
// Score: (0, 2)
// Safe: None