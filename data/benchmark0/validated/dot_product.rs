use vstd::prelude::*;

verus! {

pub open spec fn partial_dot(a: Seq<u8>, b: Seq<u8>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 }
    else { partial_dot(a, b, n - 1) + a[n - 1] as int * b[n - 1] as int }
}

proof fn lemma_partial_dot_bound(a: Seq<u8>, b: Seq<u8>, n: int)
    requires
        0 <= n <= a.len(),
        n <= b.len(),
    ensures
        0 <= partial_dot(a, b, n) <= n * 255 * 255,
    decreases n
{
    if n <= 0 {
    } else {
        lemma_partial_dot_bound(a, b, n - 1);
        assert(0 <= a[n - 1] as int <= 255);
        assert(0 <= b[n - 1] as int <= 255);
        let x = a[n - 1] as int;
        let y = b[n - 1] as int;
        assert(x * y <= 255 * 255) by (nonlinear_arith)
            requires 0 <= x <= 255, 0 <= y <= 255;
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
            0 <= i <= a.len(),
            a@.len() == b@.len(),
            a@.len() <= 66051,
            sum as int == partial_dot(a@, b@, i as int),
            sum as int <= i as int * 255 * 255,
    {
        proof {
            lemma_partial_dot_bound(a@, b@, i as int);
        }
        let ai: u32 = a[i] as u32;
        let bi: u32 = b[i] as u32;
        
        proof {
            assert(ai as int <= 255);
            assert(bi as int <= 255);
            let x: int = ai as int;
            let y: int = bi as int;
            assert(x * y <= 255 * 255) by (nonlinear_arith)
                requires 0 <= x <= 255, 0 <= y <= 255;
            assert(sum as int + ai as int * bi as int <= i as int * 255 * 255 + 255 * 255);
            let idx: int = i as int;
            assert((idx + 1) * 255 * 255 == idx * 255 * 255 + 255 * 255) by (nonlinear_arith);
        }
        sum = sum + (ai * bi);
    }
    sum
}

} // verus!

fn main() {}
