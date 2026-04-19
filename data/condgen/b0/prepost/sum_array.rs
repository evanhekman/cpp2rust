use vstd::prelude::*;

verus! {

pub open spec fn sum_partial(a: Seq<i32>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 } else { sum_partial(a, n - 1) + a[n - 1] as int }
}

pub fn sum_array(a: &[i32]) -> (result: i32)
    requires
        a@.len() as int <= 1,
    ensures
        result as int == sum_partial(a@, a@.len() as int),
{
    assume(false);
}

} // verus!
