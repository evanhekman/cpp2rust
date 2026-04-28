use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut Vec<i32>)
    requires
        old(a)@.len() >= 1,
    ensures
        a@.len() == old(a)@.len(),
        forall|k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[a@.len() - 1 - k],
{
    let ghost old_a = a@;
    let len = a.len();
    let mut lo: usize = 0;
    let mut hi: usize = len - 1;
    
    while lo < hi
        invariant
            0 <= lo,
            lo + hi == len - 1,
            hi < len,
            a@.len() == len,
            len == old_a.len(),
            forall|k: int| lo <= k && k <= hi ==> a@[k] == old_a[k],
            forall|k: int| 0 <= k && k < lo ==> a@[k] == old_a[len - 1 - k],
            forall|k: int| hi < k && k < len ==> a@[k] == old_a[len - 1 - k],
    {
        let tmp_lo: i32 = a[lo];
        let tmp_hi: i32 = a[hi];
        a.set(lo, tmp_hi);
        a.set(hi, tmp_lo);
        lo = lo + 1;
        hi = hi - 1;
    }
}

fn main() {}

} // verus!