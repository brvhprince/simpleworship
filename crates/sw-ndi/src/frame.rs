//! Pictures, and the pixel arithmetic for fades.

/// A picture: `width × height` pixels, 4 bytes each in the order blue,
/// green, red, alpha, with the colour already multiplied by the alpha
/// ("premultiplied"), rows one after another with no padding.
///
/// Premultiplied pixels blend correctly: a fade is a plain average of two
/// pictures. NDI wants straight alpha, so [`Frame::write_straight`]
/// converts when sending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, as described above.
    pub pixels: Vec<u8>,
}

impl Frame {
    /// A fully see-through picture.
    pub fn transparent(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    /// From red, green, blue, alpha bytes with straight alpha (as in a PNG).
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Self {
        let mut pixels = Vec::with_capacity(rgba.len());
        for &[r, g, b, a] in rgba.as_chunks::<4>().0 {
            pixels.extend_from_slice(&[premultiply(b, a), premultiply(g, a), premultiply(r, a), a]);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// This picture at `width × height`, if it is another size (a window
    /// that came out a few pixels smaller, say). Nearest-neighbour: only
    /// for small corrections.
    pub fn resized(self, width: u32, height: u32) -> Self {
        if self.width == width && self.height == height {
            return self;
        }
        if self.width == 0 || self.height == 0 {
            return Self::transparent(width, height);
        }
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let source_y = (u64::from(y) * u64::from(self.height) / u64::from(height)) as u32;
            for x in 0..width {
                let source_x = (u64::from(x) * u64::from(self.width) / u64::from(width)) as u32;
                let at = ((source_y * self.width + source_x) * 4) as usize;
                pixels.extend_from_slice(&self.pixels[at..at + 4]);
            }
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Writes `from` faded into `to` (`amount` 0 is all `from`, 1 is all
    /// `to`) into `out`, premultiplied. Both must be the same size.
    pub fn mix_into(from: &Frame, to: &Frame, amount: f32, out: &mut Vec<u8>) {
        let weight = (amount.clamp(0.0, 1.0) * 256.0) as u32;
        out.clear();
        out.extend(
            from.pixels.iter().zip(&to.pixels).map(|(&a, &b)| {
                ((u32::from(a) * (256 - weight) + u32::from(b) * weight) >> 8) as u8
            }),
        );
    }

    /// Writes premultiplied `pixels` into `out` with straight alpha, as NDI
    /// wants them (still blue, green, red, alpha).
    pub fn write_straight(pixels: &[u8], out: &mut Vec<u8>) {
        out.clear();
        out.reserve(pixels.len());
        for pixel in pixels.as_chunks::<4>().0 {
            let a = pixel[3];
            out.extend_from_slice(&[
                unpremultiply(pixel[0], a),
                unpremultiply(pixel[1], a),
                unpremultiply(pixel[2], a),
                a,
            ]);
        }
    }
}

fn premultiply(colour: u8, alpha: u8) -> u8 {
    ((u32::from(colour) * u32::from(alpha) + 127) / 255) as u8
}

fn unpremultiply(colour: u8, alpha: u8) -> u8 {
    match alpha {
        0 => 0,
        255 => colour,
        _ => ((u32::from(colour) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(frame: &Frame, index: usize) -> [u8; 4] {
        let at = index * 4;
        frame.pixels[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn straight_alpha_round_trips() {
        // Half-transparent red, solid green, fully transparent.
        let rgba = [255, 0, 0, 128, 0, 255, 0, 255, 10, 20, 30, 0];
        let frame = Frame::from_rgba(3, 1, &rgba);
        assert_eq!(pixel(&frame, 0), [0, 0, 128, 128]);
        assert_eq!(pixel(&frame, 1), [0, 255, 0, 255]);
        assert_eq!(pixel(&frame, 2), [0, 0, 0, 0]);

        let mut straight = Vec::new();
        Frame::write_straight(&frame.pixels, &mut straight);
        assert_eq!(&straight[0..4], &[0, 0, 255, 128]);
        assert_eq!(&straight[4..8], &[0, 255, 0, 255]);
        assert_eq!(&straight[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn a_fade_goes_from_one_picture_to_the_other() {
        let clear = Frame::transparent(1, 1);
        let white = Frame {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        };
        let mut out = Vec::new();
        Frame::mix_into(&clear, &white, 0.0, &mut out);
        assert_eq!(out, vec![0; 4]);
        Frame::mix_into(&clear, &white, 0.5, &mut out);
        assert_eq!(out, vec![127; 4]);
        Frame::mix_into(&clear, &white, 1.0, &mut out);
        assert_eq!(out, vec![255; 4]);
    }

    #[test]
    fn small_size_differences_are_corrected() {
        let frame = Frame::from_rgba(2, 1, &[255, 0, 0, 255, 0, 0, 255, 255]).resized(4, 2);
        assert_eq!((frame.width, frame.height), (4, 2));
        assert_eq!(pixel(&frame, 0), [0, 0, 255, 255]);
        assert_eq!(pixel(&frame, 3), [255, 0, 0, 255]);
        assert_eq!(pixel(&frame, 7), [255, 0, 0, 255]);
    }
}
