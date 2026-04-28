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
            0 <= i <= a.len(),
            a@.len() >= 1,
            a@.len() <= usize::MAX / 2,
            m == a@[0] 
                || exists|k: int| (0 <= k && k < i && k % 2 == 0) && m == a@[k],
            forall|j: int| (0 <= j && j < i && j % 2 == 0) ==> a@[j] <= m,
            i % 2 == 0,
        decreases a.len() - i
    {
        if a[i] > m {
            m = a[i];
        }
        i += 2;
    }
    m
}
}

// Score: (0, 3)
// Safe: True