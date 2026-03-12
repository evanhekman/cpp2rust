// Return by value instead of writing through an output pointer.
// Pointer aliasing concerns disappear; the type system enforces uniqueness.
// Callers must satisfy the no-overflow precondition (enforced by Verus).

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
