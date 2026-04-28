use vstd::prelude::*;
fn main() {}
verus! {

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
    let n = a.len();
    while lo < hi
        invariant
            old(a)@.len() >= 1,
            a@.len() == n,
            n == old(a)@.len(),
            lo <= n,
            hi < n,
            lo <= hi + 1,
            forall|k: int| 0 <= k && k < lo as int ==> a@[k] == old(a)@[n as int - 1 - k],
            forall|k: int| hi as int < k && k < n as int ==> a@[k] == old(a)@[n as int - 1 - k],
            forall|k: int|
                lo as int <= k && k <= hi as int ==>
                    a@[k] == old(a)@[k],
        decreases (hi as int + 1 - lo as int)
    {
        let old_lo = lo;
        let old_hi = hi;

        let tmp = a[hi];
        a[hi] = a[lo];
        a[lo] = tmp;

        lo += 1;
        hi -= 1;
    }
}

} // verus!
// Score: (0, 1)
// Safe: False