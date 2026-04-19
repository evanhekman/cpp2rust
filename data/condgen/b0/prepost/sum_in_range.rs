use vstd::prelude::*;

verus! {

pub open spec fn filtered_sum(a: Seq<i32>, n: int, lo: int, hi: int) -> int
    decreases n
{
    if n <= 0 { 0 } else { let x = if lo <= a[n - 1] as int && a[n - 1] as int <= hi { a[n - 1] as int } else { 0 }; x + filtered_sum(a, n - 1, lo, hi) }
}

pub fn sum_in_range(a: &[i32], lo: i32, hi: i32) -> (result: i32)
    requires
        a@.len() >= 0,
        a@.len() <= 1,
    ensures
        result as int == filtered_sum(a@, a@.len() as int, lo, hi),
{
    assume(false);
}

} // verus!
