pub fn reverse(a: &mut [i32]) {
    if a.len() == 0 { return; }
    let mut lo = 0usize;
    let mut hi = a.len() - 1;
    while lo < hi {
        a.swap(lo, hi);
        lo += 1;
        hi -= 1;
    }
}
