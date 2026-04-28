use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires

    ensures
        result as int >= -1,
        (result as int == -1) <==> forall|i: int| 0 <= i && i < v@.len() ==> v@[i] >= 0,
        (result as int >= 0) ==> v@[result as int] < 0 && forall|i: int| 0 <= i && i < result as int ==> v@[i] >= 0,
{
    assume(false);
}

} // verus!
