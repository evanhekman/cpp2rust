use vstd::prelude::*;

verus! {

pub fn first_negative(v: &[i32]) -> (result: i32)
    requires
        v.len() <= i32::MAX as usize,
    ensures
        result == -1 || (0 <= result as int < v@.len() && v@[result as int] < 0),
        result >= 0 ==> forall|j: int| 0 <= j < result ==> v@[j] >= 0,
        result == -1 ==> forall|j: int| 0 <= j < v@.len() ==> v@[j] >= 0,
{
    let mut i: usize = 0;
    while i < v.len()
        invariant
            // i stays within bounds and within i32 range (so cast is safe)
            i <= v.len(),
            i <= i32::MAX as usize,
            // every element before i is non-negative
            forall|j: int| 0 <= j < i as int ==> v@[j] >= 0,
    {
        if v[i] < 0 {
            return i as i32;
        }
        i += 1;
    }
    // Loop exited normally: i == v.len(), so all elements are non-negative
    -1
}

} // verus!
