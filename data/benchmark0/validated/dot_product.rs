use vstd::prelude::*;

verus! {

pub open spec fn partial_dot(a: Seq<u8>, b: Seq<u8>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 }
    else { partial_dot(a, b, n - 1) + a[n - 1] as int * b[n - 1] as int }
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
            i <= a@.len(),
            a@.len() == b@.len(),
            a@.len() <= 66051,
            sum as int == partial_dot(a@, b@, i as int),
            sum as int <= 65025 * i as int,
    {
        assert(a@[i as int] as int * b@[i as int] as int <= 65025) by (nonlinear_arith) {
            assert(a@[i as int] as int <= 255);
            assert(b@[i as int] as int <= 255);
        };

        let term = (a[i] as u32) * (b[i] as u32);

        sum += term;
    }

    sum
}

} // verus!
