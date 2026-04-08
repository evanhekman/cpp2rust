use vstd::prelude::*;

verus! {

pub open spec fn dot_product(a: Seq<u8>, b: Seq<u8>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 } else { dot_product(a, b, n - 1) + (a[n - 1] as int * b[n - 1] as int) }
}

pub fn dot(a: &[u8], b: &[u8]) -> (result: i32)
    requires
        a@.len() == b@.len(),
        a@.len() <= 33025,
    ensures
        result as int == dot_product(a@, b@, a@.len() as int),
{
    assume(false);
}

} // verus!
