
use vstd::prelude::*;

verus! {

#[verifier::loop_isolation(false)]

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

    // Assertion before the loop to satisfy the invariant
    proof {
        assert(forall |k: int| 0 <= k < a.len() && k % 2 == 0 ==> a[k] <= m);
        assert(exists |k: int| 0 <= k < a.len() && k % 2 == 0 && a[k] == m);
    }

    while i < a.len() 
        invariant
            forall |k: int| 0 <= k < a.len() && k % 2 == 0 ==> a[k] <= m,
            exists |k: int| 0 <= k < a.len() && k % 2 == 0 && a[k] == m // Corrected by AI
    {
        if a[i] > m { 
            m = a[i];
        }
        proof {
            assert(forall |k: int| 0 <= k <= i && k % 2 == 0 ==> a[k] <= m);
            assert(exists |k: int| 0 <= k <= i && k % 2 == 0 && a[k] == m);
        }
        i += 2;
    }
    m
}

fn main() {}

}

// Score: (1, 2)
// Safe: True