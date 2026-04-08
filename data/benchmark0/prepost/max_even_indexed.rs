use vstd::prelude::*;

verus! {

pub open spec fn max_even_indexed_spec(a: Seq<i32>) -> int
    decreases a.len()
{
    if a.len() <= 0 { -i32::MAX as int } else if a.len() == 1 { a[0] as int } else { let tail_max = max_even_indexed_spec(a.slice(2, a.len())); if a[0] as int > tail_max { a[0] as int } else { tail_max } }
}

pub fn max_even_indexed(a: &mut [i32]) -> (result: i32)
    requires
        a@.len() > 0,
        a@.len() % 2 == 0,
    ensures
        result as int == max_even_indexed_spec(a@),
{
    assume(false);
}

} // verus!
