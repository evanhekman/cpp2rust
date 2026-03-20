
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

proof fn lemma_partial_dot_monotonic(a: Seq<u8>, b: Seq<u8>, n: int)
    requires
        a.len() == b.len(),
        0 <= n <= a.len(),
    ensures
        partial_dot(a, b, n) <= partial_dot(a, b, ( a.len() ) as int),
    decreases n
{
    if n < a.len() {
        lemma_partial_dot_monotonic(a, b, n + 1);
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
    assert(partial_dot(a@, b@, 0) <= partial_dot(a@, b@, a@.len() as int)); // Added by AI
    assert((sum as u64) + ((a[0] as u32) * (b[0] as u32)) as u64 <= u32::MAX as u64); // Added by AI
    for i in 0..a.len()
        invariant
            a@.len() == b@.len(), // Added by AI for assertion fail
            a@.len() <= 66051,
            partial_dot(a@, b@, i as int) <= partial_dot(a@, b@, a@.len() as int),
            sum as int == partial_dot(a@, b@, i as int),
            (sum as u64) <= u32::MAX as u64,
            i < a.len() ==> (a[(i) as int] as u32) * (b[(i) as int] as u32) <= u32::MAX as u32, // Fixed invariant
    {
        assert((a[(i) as int] as u32) * (b[(i) as int] as u32) <= u32::MAX as u32);
        sum += (a[i] as u32) * (b[i] as u32);

        // Use the lemma to maintain the loop invariant
        proof {
            assert(a@.len() == b@.len());
        } // Added by AI
        assert(partial_dot(a@, b@, i as int + 1) <= partial_dot(a@, b@, a@.len() as int)) by {
            lemma_partial_dot_monotonic(a@, b@, i as int + 1);
        };
    }
    sum
}

fn main() {} // Added by AI

} // verus!

// Score: (2, 5)
// Safe: True