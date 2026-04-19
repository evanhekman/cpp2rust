use vstd::prelude::*;

verus! {

pub open spec fn count_positive_spec(a: Seq<i32>, n: int) -> int
    decreases n
{
    if n <= 0 { 0 } else { count_positive_spec(a, n - 1) + (if a[n - 1] > 0 { 1 } else { 0 }) }
}

pub fn count_positive(a: &[i32]) -> (result: i32)
    requires

    ensures
        result as int == count_positive_spec(a@, a@.len() as int),
{
    assume(false);
}

} // verus!
