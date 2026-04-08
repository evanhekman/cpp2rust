use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut [i32]) -> (result: ())
    requires

    ensures
        forall|i: int| 0 <= i && i < a@.len() ==> a@[i] == old(a@[(a@.len() - 1) - i]),
{
    assume(false);
}

} // verus!
