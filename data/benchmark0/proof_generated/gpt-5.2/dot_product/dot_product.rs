use vstd::prelude::*;
fn main() {}
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
            a@.len() == b@.len(),
            a@.len() <= 66051,
            a.len() == a@.len(),
            b.len() == b@.len(),
            i <= a.len(),
            0 <= i as int <= a@.len(),
            sum as int == partial_dot(a@, b@, i as int),
            0 <= sum as int,
            partial_dot(a@, b@, i as int) <= u32::MAX as int,
        decreases a.len() - i
    {
        assert(i < a.len());
        assert(i < a@.len());

        assert(partial_dot(a@, b@, (i + 1) as int)
            == partial_dot(a@, b@, i as int) + a[i] as int * b[i] as int);

        assert(partial_dot(a@, b@, (i + 1) as int) <= u32::MAX as int) by (nonlinear_arith) {
            assert(a@.len() <= 66051);
            assert(i < a@.len());
            assert(partial_dot(a@, b@, i as int) <= u32::MAX as int);
            assert(a[i] as int * b[i] as int <= 255int * 255int) by (nonlinear_arith) { };
        };

        sum += (a[i] as u32) * (b[i] as u32);
    }
    sum
}

} // verus!
// Score: (0, 4)
// Safe: True