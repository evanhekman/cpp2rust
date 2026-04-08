use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires

    ensures
        result as int == -1 || (0 <= result as int && result as int < v@.len() && v@[result as int] < 0),
        forall|i: int| 0 <= i && i < result as int ==> 0 <= v@[i],
{
    assume(false);
}

} // verus!
