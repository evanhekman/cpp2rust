use vstd::prelude::*;

verus! {

// Pre:  at least one element (index 0 is always even-indexed)
// Post: result is an upper bound for all even-indexed elements,
//       and equals some even-indexed element (i.e. it is achievable)
pub fn max_even_indexed(a: &[i32]) -> (result: i32)
    requires
        a@.len() >= 1,
        a@.len() <= usize::MAX / 2,
    ensures
        forall|i: int| (0 <= i && i < a@.len() && i % 2 == 0) ==> a@[i] <= result,
        exists|i: int| (0 <= i && i < a@.len() && i % 2 == 0) && a@[i] == result,
{
    let mut m = a[0];
    let mut i = 2usize;
    while i < a.len() {
        if a[i] > m { m = a[i]; }
        i += 2;
    }
    m
}

} // verus!
