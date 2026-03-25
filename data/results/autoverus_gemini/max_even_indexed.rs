use vstd::prelude::*;

verus! {

#[verifier::loop_isolation(false)]

// Pre:  at least one element (index 0 is always even-indexed)
// Post: result is an upper bound for all even-indexed elements,
//       and equals some even-indexed element (i.e. it is achievable)
pub fn max_even_indexed(a: &[i32]) -> (result: i32)
    requires
        a@.len() >= 1,
        a@.len() <= usize::MAX / 2,
    ensures
        forall|i: int| (0 <= i && i < a@.len() && i % 2 == 0) ==> a@[i] <= result,
        exists|i: int| (0 <= i && i < a@.len() && i % 2 == 0) && a@[i] == result,
{
    let mut m = a[0];
    let mut i = 2usize;
    while i < a.len()
        invariant
            forall|j: int| (0 <= j && j < a@.len() && j % 2 == 0) ==> a@[j] <= m,
    {
        if a[i] > m { m = a[i]; }
        i += 2;
    }
    // After the loop, 'm' holds the maximum value among the even-indexed elements encountered.
    // The final invariant `forall|j: int| (0 <= j && j < a@.len() && j % 2 == 0) ==> a@[j] <= m`
    // ensures that 'm' is an upper bound for all even-indexed elements.
    // The initialization `m = a[0]` and the update `if a[i] > m { m = a[i]; }`
    // along with the loop termination condition ensure that 'm' is achievable.
    m
}

// Add a main function to make the crate runnable.
fn main() {}

} // verus!

// Score: (1, 2)
// Safe: True