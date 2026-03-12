use vstd::prelude::*;

verus! {

pub fn clamp_all(v: &mut [i32], lo: i32, hi: i32)
    requires
        lo <= hi,
    ensures
        v@.len() == old(v)@.len(),
        forall|i: int| 0 <= i && i < v@.len() ==> lo <= v@[i] && v@[i] <= hi,
{
    let mut i = 0usize;

    while i < v.len()
        invariant
            i <= v@.len(),
            v@.len() == old(v)@.len(),
            lo <= hi,
            forall|j: int| 0 <= j && j < i as int ==> lo <= v@[j] && v@[j] <= hi,
        decreases v@.len() - i,
    {
        if v[i] < lo { v[i] = lo; }
        else if v[i] > hi { v[i] = hi; }
        i += 1;
    }
}

} // verus!
