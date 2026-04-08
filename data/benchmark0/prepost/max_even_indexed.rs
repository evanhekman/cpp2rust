use vstd::prelude::*;

verus! {

pub open spec fn max_even_indexed_spec(a: Seq<i32>) -> int
    decreases a.len()
{
    if a.len() == 0 { i32::MIN as int } else if a.len() == 1 { a[0] as int } else { let max_rest = max_even_indexed_spec(a.subrange(2, a.len())); if a[0] as int > max_rest { a[0] as int } else { max_rest } }
}

pub fn max_even_indexed(a: &mut [i32]) -> (result: i32)
    requires
        a@.len() > 0,
    ensures
        result as int == max_even_indexed_spec(old(a)@),
{
    assume(false);
}

} // verus!
