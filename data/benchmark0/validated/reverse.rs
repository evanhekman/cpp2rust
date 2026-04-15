use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut Vec<i32>)
    requires
        old(a)@.len() >= 1,
    ensures
        a@.len() == old(a)@.len(),
        forall|k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[a@.len() - 1 - k],
{
    let n = a.len();
    if n <= 1 {
        return;
    }
    
    let mut lo: usize = 0;
    let mut hi: usize = n - 1;
    let ghost old_a = a@;
    
    while lo < hi
        invariant
            lo <= hi + 1,
            lo <= n,
            hi < n,
            a.len() == n,
            n == old_a.len(),
            lo as int + hi as int == n as int - 1,
            forall|k: int| #![auto] 0 <= k < lo ==> a@[k] == old_a[n as int - 1 - k],
            forall|k: int| #![auto] hi < k < n ==> a@[k] == old_a[n as int - 1 - k],
            forall|k: int| #![auto] lo <= k <= hi ==> a@[k] == old_a[k],
    {
        let tmp = a[lo];
        a.set(lo, a[hi]);
        a.set(hi, tmp);
        
        lo = lo + 1;
        hi = hi - 1;
    }
}

fn main() {}

} // verus!