/// Compute grayscale luminance from RGB using BT.601 coefficients.
#[inline]
fn luminance(r: u8, g: u8, b: u8) -> u8 {
    // Fixed-point arithmetic: 0.299*R + 0.587*G + 0.114*B
    // Multiply by 256, then shift right 8
    ((77 * r as u32 + 150 * g as u32 + 29 * b as u32) >> 8) as u8
}

/// Apply binary threshold to RGBA pixel data in-place.
/// Pixels with luminance >= threshold become white, otherwise black.
/// Alpha channel is preserved at 255.
pub fn apply_threshold(rgba: &mut [u8], threshold: u8) {
    for pixel in rgba.chunks_exact_mut(4) {
        let lum = luminance(pixel[0], pixel[1], pixel[2]);
        let val = if lum >= threshold { 255 } else { 0 };
        pixel[0] = val;
        pixel[1] = val;
        pixel[2] = val;
        pixel[3] = 255;
    }
}
