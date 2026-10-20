// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! Decoded image buffers.
//!
//! One pixel format almost everywhere: 8-bit RGBA, straight (non-premultiplied)
//! alpha, gamma-encoded Rec. 709, rows packed tightly with no padding. Every
//! encoder converts from it, and every consumer but the compositor's upload
//! reads it.
//!
//! The exception is a deep frame ([`Depth::Sixteen`]): sixteen bits a
//! channel, still RGBA, in the source's own signal ([`Signal`]) - an HDR clip
//! as it was shot, not tone-mapped to eight bits on the CPU. The decoder
//! makes one only when a caller asks (`DecodeOptions::deep`) and the source
//! is wide, and the only caller that asks is the one whose frames go
//! straight to the GPU, which brings them into its working space as it
//! uploads them. A second type would have run through the frame pool, the
//! plan and every cache for one reader, so a deep frame is the same type
//! as any other and the accessors below know its depth: a frame is read
//! and written whole, and the operations that work in eight bits a channel
//! - the filter chain, a copy between depths - say so rather than guess.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes per pixel of an eight-bit frame, the format nearly everything uses.
pub const BYTES_PER_PIXEL: usize = 4;

/// The widest or tallest a frame may be, a side: 8K. The downlevel
/// texture limit every GPU meets is this, so a frame past it cannot be
/// drawn, only asked for; a document, a request and a preview are all held
/// to it before a texture is made (audit 2026-10-09, finding 8).
pub const MAX_SIDE: u32 = 8192;

/// How many bits a channel a frame's pixels hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Depth {
    /// Eight: `u8` channels, four bytes a pixel.
    #[default]
    Eight,
    /// Sixteen: little-endian `u16` channels, eight bytes a pixel.
    Sixteen,
}

/// What a frame's RGB values mean: the transfer they are encoded with and
/// the primaries they are on. An eight-bit frame is always [`Signal::Sdr`];
/// a deep one carries its source's.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Signal {
    /// Gamma-encoded (BT.1886, 2.4) on Rec. 709 primaries.
    #[default]
    Sdr,
    /// Gamma-encoded (BT.1886, 2.4) on Rec. 2020 primaries: a wide-gamut
    /// SDR source.
    SdrWide,
    /// ARIB STD-B67 hybrid log-gamma on Rec. 2020: an iPhone's HDR.
    Hlg,
    /// SMPTE ST 2084 perceptual quantiser on Rec. 2020: HDR10.
    Pq,
}

/// The next frame identity; see [`Frame::id`].
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn mint() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// An RGBA8 image - or an RGBA16 one, for a deep frame.
///
/// Width and height are fixed at construction, and the buffer is always
/// exactly `width * height` pixels of the frame's own depth - four bytes
/// each, or eight for a deep one - so indexing arithmetic cannot go wrong
/// after the fact. The accessors below read the frame's own depth, not a
/// fixed four bytes a pixel: an offset for four would land in the middle
/// of a deep picture, and a write there would take half of a pixel with
/// it.
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    /// Which picture this is; see [`Frame::id`].
    id: u64,
    /// Eight bits a channel, or sixteen for a deep frame.
    depth: Depth,
    /// What the values mean.
    signal: Signal,
}

impl Clone for Frame {
    /// A copy is a picture of its own: it may be changed next, and a
    /// renderer holding the original must not take it for the copy.
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
            id: mint(),
            depth: self.depth,
            signal: self.signal,
        }
    }
}

impl PartialEq for Frame {
    /// The same picture, whatever its identity.
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.depth == other.depth
            && self.signal == other.signal
            && self.pixels == other.pixels
    }
}

impl Eq for Frame {}

impl Frame {
    /// A number no other frame has had, and this one loses the moment its
    /// pixels are touched: a renderer keys what it has uploaded by it, so a
    /// picture it already holds - a still, a title, a paused clip's frame
    /// - is not uploaded again. Never zero.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// How many bytes a frame of this size occupies, eight bits a channel.
    /// A deep frame is twice that; see [`Frame::bytes_per_pixel`].
    pub const fn byte_len(width: u32, height: u32) -> usize {
        width as usize * height as usize * BYTES_PER_PIXEL
    }

    /// An opaque black frame.
    pub fn black(width: u32, height: u32) -> Self {
        let mut pixels = vec![0u8; Self::byte_len(width, height)];
        for pixel in pixels.chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel[3] = 255;
        }
        Self {
            width,
            height,
            pixels,
            id: mint(),
            depth: Depth::Eight,
            signal: Signal::Sdr,
        }
    }

    /// A fully transparent frame.
    pub fn transparent(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; Self::byte_len(width, height)],
            id: mint(),
            depth: Depth::Eight,
            signal: Signal::Sdr,
        }
    }

    /// Wraps an existing buffer.
    ///
    /// Returns `None` if the buffer is not exactly `width * height * 4` bytes.
    /// Decoders should use this rather than trusting their own arithmetic.
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        (pixels.len() == Self::byte_len(width, height)).then_some(Self {
            width,
            height,
            pixels,
            id: mint(),
            depth: Depth::Eight,
            signal: Signal::Sdr,
        })
    }

    /// Wraps a deep buffer: little-endian `u16` RGBA, exactly `width *
    /// height * 8` bytes, in `signal`. `None` if the size is wrong.
    pub fn from_rgba64(width: u32, height: u32, pixels: Vec<u8>, signal: Signal) -> Option<Self> {
        (pixels.len() == Self::byte_len(width, height) * 2).then_some(Self {
            width,
            height,
            pixels,
            id: mint(),
            depth: Depth::Sixteen,
            signal,
        })
    }

    /// Eight bits a channel, or sixteen.
    pub const fn depth(&self) -> Depth {
        self.depth
    }

    /// What the values mean: see [`Signal`].
    pub const fn signal(&self) -> Signal {
        self.signal
    }

    /// Bytes a pixel of this frame takes.
    pub const fn bytes_per_pixel(&self) -> usize {
        match self.depth {
            Depth::Eight => BYTES_PER_PIXEL,
            Depth::Sixteen => BYTES_PER_PIXEL * 2,
        }
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// True if both frames have the same dimensions.
    pub const fn same_size_as(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }

    /// The raw RGBA bytes, row-major from the top-left.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The raw RGBA bytes, mutably. A new identity with them: whatever is
    /// written makes this a different picture from the one uploaded.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        self.id = mint();
        &mut self.pixels
    }

    /// Consumes the frame and yields its buffer, so it can be handed to an
    /// encoder without a copy.
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }

    /// Byte offset of the pixel at `(x, y)`, or `None` if out of bounds.
    ///
    /// The frame's own depth: a deep frame's pixels are eight bytes each,
    /// and an offset computed for four would land in the middle of the
    /// picture.
    pub const fn offset_of(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * self.bytes_per_pixel())
    }

    /// Reads one pixel as `[r, g, b, a]`: a deep frame's to eight bits, its
    /// channels' high bytes, for a test or a log to look at.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let offset = self.offset_of(x, y)?;
        Some(match self.depth {
            Depth::Eight => [
                self.pixels[offset],
                self.pixels[offset + 1],
                self.pixels[offset + 2],
                self.pixels[offset + 3],
            ],
            Depth::Sixteen => [
                self.pixels[offset + 1],
                self.pixels[offset + 3],
                self.pixels[offset + 5],
                self.pixels[offset + 7],
            ],
        })
    }

    /// Writes one pixel. Out-of-bounds writes are ignored.
    ///
    /// The colour is eight bits a channel: a deep frame takes it as such
    /// and spreads it over sixteen, so that reading the pixel back gives
    /// the same four numbers again.
    ///
    /// The spread is of *code values*, not of light. A deep frame holds
    /// its own transfer function - PQ, HLG - and `255` in it is that
    /// function's maximum code, not white; and a deep frame can hold
    /// values above 1.0 that this signature cannot express at all. So
    /// the write is only meaningful for a frame in a linear signal, and
    /// [`Frame::pixel`] reads a deep frame back truncated to its high
    /// byte.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        self.id = mint();
        let Some(offset) = self.offset_of(x, y) else {
            return;
        };
        match self.depth {
            Depth::Eight => {
                self.pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&rgba);
            }
            Depth::Sixteen => {
                // Eight bits a channel spread over sixteen: the value a
                // channel of `v / 255` takes in sixteen bits, so the high
                // byte - the one `pixel` reads - is `v` again.
                for (index, channel) in rgba.iter().enumerate() {
                    let wide = u16::from(*channel) * 257;
                    let at = offset + index * 2;
                    self.pixels[at] = (wide & 0xff) as u8;
                    self.pixels[at + 1] = (wide >> 8) as u8;
                }
            }
        }
    }

    /// Copies `source` into this frame with its top-left corner at (`x`, `y`),
    /// clipping whatever falls outside. No blending: the source's pixels
    /// replace what was there.
    ///
    /// A source of another depth is not copied at all. The two depths are
    /// not two sizes of the same colour: a deep frame holds code values
    /// in its own transfer function - PQ, HLG - and there is no spreading
    /// of an eight-bit number that means the same thing there. Copying
    /// one into the other would hand the caller a picture that looks
    /// right and is not, so the copy is refused instead.
    pub fn blit(&mut self, source: &Frame, x: u32, y: u32) {
        self.id = mint();
        let width = self.width;
        let height = self.height;
        let columns = source.width.min(width.saturating_sub(x)) as usize;
        let rows = source.height.min(height.saturating_sub(y)) as usize;
        if columns == 0 || rows == 0 {
            return;
        }
        if source.depth != self.depth {
            return;
        }
        let source_row = source.width as usize * source.bytes_per_pixel();
        let target_bytes = self.bytes_per_pixel();
        let target_row = width as usize * target_bytes;
        let row_bytes = columns * target_bytes;
        for row in 0..rows {
            let from = row * source_row;
            let to = (y as usize + row) * target_row + x as usize * target_bytes;
            self.pixels[to..to + row_bytes].copy_from_slice(&source.pixels[from..from + row_bytes]);
        }
    }

    /// Fills the whole frame with one colour.
    pub fn fill(&mut self, rgba: [u8; 4]) {
        self.id = mint();
        // The frame's own depth, hoisted: the arm below borrows the pixels
        // mutably, and the borrow of the whole frame would not live with it.
        let bytes = self.bytes_per_pixel();
        match self.depth {
            Depth::Eight => {
                for pixel in self.pixels.chunks_exact_mut(BYTES_PER_PIXEL) {
                    pixel.copy_from_slice(&rgba);
                }
            }
            Depth::Sixteen => {
                for pixel in self.pixels.chunks_exact_mut(bytes) {
                    for (index, channel) in rgba.iter().enumerate() {
                        let wide = u16::from(*channel) * 257;
                        pixel[index * 2] = (wide & 0xff) as u8;
                        pixel[index * 2 + 1] = (wide >> 8) as u8;
                    }
                }
            }
        }
    }

    /// Iterates over rows, top to bottom. A frame with no width has no
    /// rows, and yields none.
    pub fn rows(&self) -> impl Iterator<Item = &[u8]> {
        let row = self.width as usize * self.bytes_per_pixel();
        self.pixels.chunks_exact(row.max(1))
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never dump several megabytes of pixels into a log line.
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.pixels.len())
            .field("depth", &self.depth)
            .field("signal", &self.signal)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_is_opaque() {
        let frame = Frame::black(2, 2);
        assert_eq!(frame.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(frame.pixels().len(), 16);
    }

    #[test]
    fn transparent_is_clear() {
        assert_eq!(Frame::transparent(1, 1).pixel(0, 0), Some([0, 0, 0, 0]));
    }

    #[test]
    fn rejects_a_buffer_of_the_wrong_size() {
        assert!(Frame::from_rgba(2, 2, vec![0; 15]).is_none());
        assert!(Frame::from_rgba(2, 2, vec![0; 16]).is_some());
    }

    #[test]
    fn out_of_bounds_access_is_none_not_a_panic() {
        let mut frame = Frame::black(2, 2);
        assert_eq!(frame.pixel(2, 0), None);
        assert_eq!(frame.pixel(0, 2), None);
        frame.set_pixel(99, 99, [255, 0, 0, 255]); // ignored
    }

    #[test]
    fn set_and_read_a_pixel() {
        let mut frame = Frame::black(4, 3);
        frame.set_pixel(3, 2, [1, 2, 3, 4]);
        assert_eq!(frame.pixel(3, 2), Some([1, 2, 3, 4]));
        assert_eq!(frame.pixel(2, 3), None);
    }

    #[test]
    fn rows_are_tightly_packed() {
        let frame = Frame::black(5, 4);
        assert_eq!(frame.rows().count(), 4);
        assert!(frame.rows().all(|row| row.len() == 20));
    }

    /// A deep frame's pixels are eight bytes each, and every accessor has
    /// to know it: an offset computed for four bytes a pixel lands in the
    /// middle of the picture, and a write there takes half of a pixel
    /// with it - the frame reads back as a darkened, half-written one.
    #[test]
    fn a_deep_frame_is_read_and_written_in_its_own_depth() {
        let mut frame = Frame::from_rgba64(3, 2, vec![0; 3 * 2 * 8], Signal::Hlg)
            .expect("a deep frame of the right size");
        assert_eq!(frame.bytes_per_pixel(), 8);
        // The rows are the frame's own rows, and not twice as many.
        assert_eq!(frame.rows().count(), 2);
        assert!(frame.rows().all(|row| row.len() == 3 * 8));
        // A pixel set and read back is the one that was set.
        frame.set_pixel(2, 1, [10, 20, 30, 40]);
        assert_eq!(frame.pixel(2, 1), Some([10, 20, 30, 40]));
        // And the pixel beside it is untouched: the write did not take
        // half of it with it.
        assert_eq!(frame.pixel(1, 1), Some([0, 0, 0, 0]));
        // A fill reaches every channel, alpha among them.
        frame.fill([1, 2, 3, 4]);
        assert_eq!(frame.pixel(0, 0), Some([1, 2, 3, 4]));
        assert_eq!(frame.pixel(2, 1), Some([1, 2, 3, 4]));
    }

    /// A copy of a deep frame lands in the deep frame's own pixels.
    #[test]
    fn a_deep_frame_is_copied_whole() {
        let mut source = Frame::from_rgba64(2, 2, vec![0; 2 * 2 * 8], Signal::Hlg)
            .expect("a deep frame of the right size");
        source.set_pixel(1, 1, [7, 8, 9, 10]);
        let mut target = Frame::from_rgba64(4, 4, vec![0; 4 * 4 * 8], Signal::Hlg)
            .expect("a deep frame of the right size");
        target.blit(&source, 2, 2);
        assert_eq!(target.pixel(3, 3), Some([7, 8, 9, 10]));
        assert_eq!(target.pixel(2, 2), Some([0, 0, 0, 0]));
    }
}
