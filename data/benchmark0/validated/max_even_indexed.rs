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
    let mut m: i32 = a[0usize];
    let mut p: usize = 0usize;
    let ghost mut witness: int = 0;
    while (p < a.len())
        invariant
            p <= a.len() + 1,
            p % 2 == 0,
            a@.len() >= 1,
            a@.len() <= usize::MAX / 2,
            forall|i: int| (0 <= i && i < p as int && i % 2 == 0 && i < a@.len()) ==> a@[i] <= m,
            0 <= witness && witness < a@.len() && witness % 2 == 0 && witness <= p as int && a@[witness] == m,
    {
        if (a[p] > m) {
            m = a[p];
            proof {
                witness = p as int;
            }
        }
        p += 2usize;
    }
    return m;
}

fn main() {}

} // verus!