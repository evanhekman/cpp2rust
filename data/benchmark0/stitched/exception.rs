use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires
        v.len() <= i32::MAX as usize,
    ensures
        result == -1 || (0 <= result as int && (result as int) < v@.len() && v@[result as int] < 0),
        result >= 0 ==> forall|j: int| 0 <= j && j < result as int ==> v@[j] >= 0,
        result == -1 ==> forall|j: int| 0 <= j && j < v@.len() ==> v@[j] >= 0,
{
    for i in 0..v.len() {
        if v[i] < 0 {
            return i as i32;
        }
    }
    -1
}

} // verus!
