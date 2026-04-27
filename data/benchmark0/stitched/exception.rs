use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires

    ensures
        result as int == -1 <==> forall|i: int| 0 <= i && i < v@.len() ==> v@[i] >= 0,
        result as int >= 0 <==> 0 <= result as int && result as int < v@.len() && v@[result as int] < 0,
{
    { for i in 0..v.len() { if (v[i] < 0) { return i; } } return -1; } 
}

} // verus!
