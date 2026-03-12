use vstd::prelude::*;

verus! {

// The requires clauses give Verus what it needs to discharge the overflow VC
// on the `a + b` expression. No additional proof hints are needed.
pub fn add(a: i32, b: i32) -> (result: i32)
    requires
        a as i64 + b as i64 <= i32::MAX as i64,
        a as i64 + b as i64 >= i32::MIN as i64,
    ensures
        result == a + b,
{
    a + b
}

} // verus!
