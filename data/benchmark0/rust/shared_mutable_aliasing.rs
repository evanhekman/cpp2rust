// Naive Rust Translation
/*
fn add_both(a: &mut i32, b: &mut i32) {
    *a += 1;
    *b += 1;
}

fn main() {
    let mut x = 0;
    add_both(&mut x, &mut x); // error
}
*/

// Unsafe Rust Translation 
/*
unsafe fn add_both(a: *mut i32, b: *mut i32) {
    *a += 1;
    *b += 1;
}

fn main() {
    let mut x = 0;
    unsafe {
        add_both(&mut x, &mut x);
    }
    println!("{}", x); // 2
}
*/

// Safe Rust Translation using Cell, which allows for interior mutability and can be used to achieve the same result as the original C code without using unsafe code. However, it is different from the original C code and the C++ translation, as it uses a different approach to achieve the same result.
/*
use std::cell::Cell;

fn add_both(a: &Cell<i32>, b: &Cell<i32>) {
    a.set(a.get() + 1);
    b.set(b.get() + 1);
}

fn main() {
    let x = Cell::new(0);
    add_both(&x, &x);
    println!("{}", x.get()); // 2
}
*/

// Safe Rust Translation, different from C++ and the original C code, but it is the only way to achieve the same result in Rust without using unsafe code.
/*
fn add_twice(x: &mut i32) {
    *x += 1;
    *x += 1;
}

fn main() {
    let mut x = 0;
    add_twice(&mut x);
    println!("{}", x); // 2
}
*/

// Safe Rust Translation, using two separate variables to achieve the same result as the original C code, but it is different from the original C code and the C++ translation.
/*
fn add_both(a: &mut i32, b: &mut i32) {
    *a += 1;
    *b += 1;
}

fn main() {
    let mut x = 0;
    let mut y = 0;
    add_both(&mut x, &mut y);
    println!("{x}, {y}"); // 1, 1
}
*/

// Safe Rust Translation, using a single variable and a boolean flag to achieve the same result as the original C code, but it is different from the original C code and the C++ translation.
/*
fn add_both_same_ok(x: &mut i32, same_twice: bool) {
    *x += 1;
    if same_twice {
        *x += 1;
    }
}

fn main() {
    let mut x = 0;
    add_both_same_ok(&mut x, true);
    println!("{x}"); // 2
}
*/

