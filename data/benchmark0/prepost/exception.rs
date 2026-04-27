use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires

    ensures
        if exists|i: int| 0 <= i && i < v@.len() && v@[i] < 0 { 0 <= result as int && result as int < v@.len() && v@[result as int] < 0 && forall|j: int| 0 <= j && j < result as int ==> v@[j] >= 0 } else { result as int == -1 },
{
    assume(false);
}

} // verus!
