use vstd::prelude::*;

verus! {

pub open spec fn partial_dot(a: Seq<u8>, b: Seq<u8>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 }
    else { partial_dot(a, b, n - 1) + a[n - 1] as int * b[n - 1] as int }
}

proof fn partial_dot_nonneg(a: Seq<u8>, b: Seq<u8>, n: int)
    requires
        0 <= n <= a.len(),
        a.len() == b.len(),
    ensures
        partial_dot(a, b, n) >= 0,
    decreases n
{
    if n <= 0 {
    } else {
        partial_dot_nonneg(a, b, n - 1);
    }
}

proof fn lemma_u8_mul_bound(x: int, y: int)
    requires
        0 <= x <= 255,
        0 <= y <= 255,
    ensures
        x * y <= 65025,
{
    assert(x * y <= 65025) by (nonlinear_arith)
        requires 0 <= x <= 255, 0 <= y <= 255;
}

proof fn lemma_mul_bound_helper(n: int)
    requires
        n >= 1,
    ensures
        (n - 1) * 65025 + 65025 == n * 65025,
{
    assert((n - 1) * 65025 + 65025 == n * 65025) by (nonlinear_arith);
}

proof fn partial_dot_bound(a: Seq<u8>, b: Seq<u8>, n: int)
    requires
        0 <= n <= a.len(),
        a.len() == b.len(),
    ensures
        partial_dot(a, b, n) <= n * 65025,
    decreases n
{
    if n <= 0 {
    } else {
        partial_dot_bound(a, b, n - 1);
        lemma_u8_mul_bound(a[n - 1] as int, b[n - 1] as int);
        lemma_mul_bound_helper(n);
    }
}

proof fn lemma_prod_bound(i: int, max_i: int)
    requires
        0 <= i < max_i,
        max_i <= 66051,
    ensures
        i * 65025 + 65025 <= 4294967295,
{
    assert(i * 65025 + 65025 <= 66050 * 65025 + 65025) by (nonlinear_arith)
        requires 0 <= i < max_i, max_i <= 66051;
    assert(66050 * 65025 + 65025 <= 4294967295) by (nonlinear_arith);
}

pub fn dot(a: &[u8], b: &[u8]) -> (result: u32)
    requires
        a@.len() == b@.len(),
        a@.len() <= 66051,
    ensures
        result as int == partial_dot(a@, b@, a@.len() as int),
{
    let mut sum: u32 = 0;
    let n = a.len();
    let mut i: usize = 0;
    while i < n
        invariant
            n == a@.len(),
            a@.len() == b@.len(),
            a@.len() <= 66051,
            0 <= i <= n,
            sum as int == partial_dot(a@, b@, i as int),
            sum as int <= (i as int) * 65025,
    {
        proof {
            partial_dot_bound(a@, b@, i as int);
            partial_dot_nonneg(a@, b@, i as int);
            lemma_prod_bound(i as int, n as int);
        }
        let ai = a[i] as u32;
        let bi = b[i] as u32;
        assert(ai <= 255);
        assert(bi <= 255);
        proof {
            lemma_u8_mul_bound(ai as int, bi as int);
        }
        let prod = ai * bi;
        sum = sum + prod;
        i = i + 1;
        proof {
            lemma_mul_bound_helper(i as int);
        }
    }
    sum
}

} // verus!

fn main() {}