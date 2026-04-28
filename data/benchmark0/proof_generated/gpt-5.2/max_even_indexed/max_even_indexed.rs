use vstd::prelude::*;
fn main() {}
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
            a@.len() >= 1,
            a@.len() <= usize::MAX / 2,
            0 <= i as int,
            i as int <= a@.len() + 1,
            i % 2 == 0,
            exists|j: int|
                0 <= j && j < a@.len() && j % 2 == 0 && j < i as int && a@[j] == m,
            forall|j: int|
                0 <= j && j < a@.len() && j % 2 == 0 && j < i as int ==> a@[j] <= m,
        decreases a@.len() - i as int + 1
    {
        if a[i] > m { m = a[i]; }
        i += 2;
    }
    m
}

} // verus!
// Score: (2, 0)
// Safe: True