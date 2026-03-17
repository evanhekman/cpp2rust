use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut [i32])
    requires
        old(a)@.len() >= 1,
    ensures
        a@.len() == old(a)@.len(),
        forall|k: int| 0 <= k && k < a@.len() ==> a@[k] == old(a)@[a@.len() - 1 - k],
{
    let ghost orig = a@;
    let n = a.len();
    let half = n / 2;
    let mut lo = 0usize;

    while lo < half
        invariant
            n == a@.len(),
            n == orig.len(),
            n >= 1,
            half == n / 2,
            lo <= half,
            // swapped positions: a@[k] == orig[n-1-k] for both ends
            forall|k: int| 0 <= k && k < lo as int ==>
                #[trigger] a@[k] == orig[orig.len() - 1 - k],
            forall|k: int| orig.len() - lo as int <= k && k < orig.len() ==>
                #[trigger] a@[k] == orig[orig.len() - 1 - k],
            // middle untouched
            forall|k: int| lo as int <= k && k < orig.len() - lo as int ==>
                #[trigger] a@[k] == orig[k],
        decreases half - lo,
    {
        let hi = n - 1 - lo;
        let tmp = a[lo];
        a[lo] = a[hi];
        a[hi] = tmp;
        lo += 1;
    }
}

} // verus!
