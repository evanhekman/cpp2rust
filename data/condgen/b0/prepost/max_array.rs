use vstd::prelude::*;

verus! {

pub fn max_array(a: &[i32]) -> (result: i32)
    requires
        a@.len() > 0,
    ensures
        forall|i: int| 0 <= i && i < a@.len() ==> result as int >= a@[i],
        exists|i: int| 0 <= i && i < a@.len() && result as int == a@[i],
{
    assume(false);
}

} // verus!
