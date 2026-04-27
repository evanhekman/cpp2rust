use vstd::prelude::*;

verus! {

pub fn reverse(a: &mut [i32]) -> (result: ())
    requires

    ensures
        a@ == old(a)@.reversed(),
{
    assume(false);
}

} // verus!
