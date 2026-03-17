// Pointer offset (a+2), stride (p+=2), and end condition (p<a+n) all collapse
// into a clean index loop. The length parameter disappears into the slice type.

pub fn max_even_indexed(a: &[i32]) -> i32 {
    let mut m = a[0];
    let mut i = 2usize;
    while i < a.len() {
        if a[i] > m { m = a[i]; }
        i += 2;
    }
    m
}
