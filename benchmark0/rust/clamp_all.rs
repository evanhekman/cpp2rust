pub fn clamp_all(v: &mut [i32], lo: i32, hi: i32) {
    for x in v.iter_mut() {
        if *x < lo { *x = lo; }
        else if *x > hi { *x = hi; }
    }
}
