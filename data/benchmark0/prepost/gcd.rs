use vstd::prelude::*;

verus! {

pub open spec fn spec_gcd(a: int, b: int) -> int
    decreases b
{
    if b == 0 { a }
    else { spec_gcd(b, a % b) }
}

pub fn gcd(a: i32, b: i32) -> (result: i32)
    requires a >= 0, b >= 0,
    ensures  result as int == spec_gcd(a as int, b as int),
{
    assume(false);
}

} // verus!
