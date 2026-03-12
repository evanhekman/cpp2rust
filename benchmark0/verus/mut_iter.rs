use vstd::prelude::*;

verus! {

pub fn remove_evens(v: &mut Vec<i32>)
    ensures
        forall|i: int| 0 <= i < v@.len() ==> v@[i] % 2 != 0,
{
    let mut i: usize = 0;

    while i < v.len()
        invariant
            // i is a valid cursor into the current vec
            i <= v@.len(),
            // every element before i is already verified odd
            forall|j: int| 0 <= j < i as int ==> v@[j] % 2 != 0,
    {
        if v[i] % 2 == 0 {
            // Remove the even element at i.
            // vstd spec: v@ == old(v@).remove(i as int)
            // Elements before i are unchanged, so the invariant is preserved.
            v.remove(i);
            // i stays the same; the next element to inspect slides into position i
        } else {
            // v[i] is odd — advance the verified prefix
            i += 1;
        }
    }
    // Loop exits when i == v@.len().
    // Invariant gives: forall j < v@.len(), v@[j] % 2 != 0  ✓
}

} // verus!
