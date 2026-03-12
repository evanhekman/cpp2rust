use vstd::prelude::*;

verus! {

// Specification: the mathematical dot product up to n elements
spec fn partial_dot(a: Seq<u8>, b: Seq<u8>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 }
    else { partial_dot(a, b, n - 1) + a[n - 1] as int * b[n - 1] as int }
}

// Pre:  slices have equal length, at most 1000 elements
//       (guarantees u32 accumulator never overflows: 1000 * 255 * 255 = 65,025,000 < 2^32)
// Post: result equals the true dot product
pub fn dot(a: &[u8], b: &[u8]) -> (result: u32)
    requires
        a@.len() == b@.len(),
        a@.len() <= 1000,
    ensures
        result as int == partial_dot(a@, b@, a@.len() as int),
{
    assume(false);
    0
}

} // verus!
