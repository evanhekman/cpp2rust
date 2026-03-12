use vstd::prelude::*;

verus! {

pub fn max_even_indexed(a: &[i32]) -> (result: i32)
    requires
        a@.len() >= 1,
    ensures
        forall|i: int| (0 <= i < a@.len() && i % 2 == 0) ==> a@[i] <= result,
        exists|i: int| (0 <= i < a@.len() && i % 2 == 0) && a@[i] == result,
{
    let mut m = a[0];
    let mut i = 2usize;

    while i < a.len()
        invariant
            i <= a@.len(),
            i % 2 == 0,
            forall|j: int| (0 <= j < i && j % 2 == 0) ==> a@[j] <= m,
            exists|j: int| (0 <= j < i && j % 2 == 0) && a@[j] == m,
    {
        if a[i] > m {
            m = a[i];
            assert(exists|j: int| (0 <= j < i + 2 && j % 2 == 0) && a@[j] == m);
        }
        i += 2;
    }

    m
}

} // verus!
