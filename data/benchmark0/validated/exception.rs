use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires

    ensures
        if exists|i: int| 0 <= i && i < v@.len() && v@[i] < 0 { result as int >= 0 && result as int < v@.len() && v@[result as int] < 0 } else { result as int == -1 },
{
    { for i in 0..v.len() { if (v[i] < 0) { return i; } } return -1; } 
}

} // verus!
