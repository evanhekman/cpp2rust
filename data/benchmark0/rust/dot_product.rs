// &[u8] slices replace pointer+length; the length parameter disappears.
// Accumulating into u32 prevents overflow: max result is 1000 * 255 * 255 = 65,025,000.
// Requires equal-length slices of at most 1000 elements.

pub fn dot(a: &[u8], b: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..a.len() {
        sum += (a[i] as u32) * (b[i] as u32);
    }
    sum
}
