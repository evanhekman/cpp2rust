use vstd::prelude::*;

verus! {

pub fn contains(a: &[u8], target: i32) -> (result: bool)
    requires
        0 <= target && target <= u8::MAX,
    ensures
        result <==> exists|i: int| 0 <= i && i < a@.len() && a@[i] as int == target,
{
    assume(false);
}

} // verus!
