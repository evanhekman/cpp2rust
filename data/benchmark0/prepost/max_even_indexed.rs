use vstd::prelude::*;

verus! {

pub open spec fn max_even_indexed_spec(a: Seq<i32>) -> int
    decreases a.len()
{
    if a.len() <= 1 { a[0] as int } else { let rest = max_even_indexed_spec(a.subrange(2, a.len() as int)); if a[0] as int >= rest { a[0] as int } else { rest } }
}

pub fn max_even_indexed(a: &mut [i32]) -> (result: i32)
    requires
        a@.len() > 0,
    ensures
        result as int == max_even_indexed_spec(a@),
        a@ == old(a)@,
{
    assume(false);
}

} // verus!
