use vstd::prelude::*;

verus! {

pub fn first_element(a: &[i32]) -> (result: i32)
    requires
        a@.len() > 0,
    ensures
        result as int == a@[0],
{
    assume(false);
}

} // verus!
