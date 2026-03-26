pub fn dot(a: &[u8], b: &[u8]) -> u32 {
    let mut sum: u32 = 0u32;
    for i in 0..a.len() {
        sum += (a[i] * b[i]);
    }
    return sum;
}
