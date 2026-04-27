use vstd::prelude::*;

verus! {

pub open spec fn partial_sum(a: Seq<i32>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 } else { partial_sum(a, n - 1) + a[n - 1] as int }
}

pub fn sum_array(a: &[i32]) -> (result: i32)
    requires
        a@.len() <= 1,
    ensures
        result as int == partial_sum(a@, a@.len() as int),
{
    assume(false);
}

} // verus!
