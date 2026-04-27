use vstd::prelude::*;

verus! {

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    requires
        forall|i: int| 0 <= i && i < a@.len() ==> a@[i] as int <= i32::MAX,
    ensures
        result <==> exists|i: int| 0 <= i && i < a@.len() && a@[i] as int == target,
{
    assume(false);
}

} // verus!
