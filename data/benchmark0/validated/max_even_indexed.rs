use vstd::prelude::*;

verus! {

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

    while i < a.len()
        invariant
            i % 2 == 0,
            i <= a@.len() + 1,
            a@.len() <= usize::MAX / 2,
            forall|j: int| (0 <= j && j < i as int && j % 2 == 0) ==> a@[j] <= m,
            exists|j: int| (0 <= j && j < a@.len() && j % 2 == 0) && a@[j] == m,
        decreases a@.len() + 1 - i,
    {
        if a[i] > m { m = a[i]; }
        i += 2;
    }
    m
}

} // verus!
