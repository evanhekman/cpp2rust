
use vstd::prelude::*;

verus! {

#[verifier::loop_isolation(false)]

pub open spec fn partial_dot(a: Seq<u8>, b: Seq<u8>, n: int) -> int
    decreases n
{
    if n <= 0 {
        0
    } else {
        partial_dot(a, b, n - 1) + a[n - 1] as int * b[n - 1] as int
    }
}

pub fn dot(a: &[u8], b: &[u8]) -> (result: u32)
    requires
        a@.len() == b@.len(),
        a@.len() <= 66051,
    ensures
        result as int == partial_dot(a@, b@, a@.len() as int),
{
    let mut sum: u32 = 0;
    for i in 0..a.len()
        invariant
            i <= a.len(),
            // The array 'a' and 'b' are not modified within this loop.
            // Therefore, the invariant can be strengthened to cover all elements up to a.len().
            forall |k: int| 0 <= k < a.len() ==> #[trigger] sum as int == partial_dot(a@, b@, i as int),
            a@.len() <= 66051,
            a@.len() == b@.len(),
            b@.len() <= 66051,
    {
        sum += (a[i] as u32) * (b[i] as u32);
    }
    sum
}

} // verus!
// Score: (0, 1)
// Safe: True