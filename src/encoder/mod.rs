//! The VP9 encoder in Rust that `docs/rust-encoder-plan.md` sets out: 8-bit
//! 4:4:4, profile 1, the subset of VP9 the plan's table names and no more.
//! Reachable only from tests until it passes the plan's gate.
//!
//! So far it writes keyframes and nothing else, and codes every block as DC
//! predicted and skipped: the frame decodes to the right size and the right
//! colour declaration, flat grey whatever the picture. That proves every
//! header bit and the partition and mode syntax against libvpx's decoder
//! before a coefficient is coded.

mod bool_writer;
mod header;
mod predict;
mod tables;
mod tile;

use crate::{Chroma, Error, MAX_THREADS, Picture, QUALITY_MAX, QUALITY_MIN, quality_to_q, tile_columns_log2};

/// The widest a tile column may be, in superblocks.
const MAX_TILE_WIDTH_B64: usize = 64;
/// The narrowest a tile column may be, in superblocks.
const MIN_TILE_WIDTH_B64: usize = 4;

/// A frame's measures in the units the bitstream counts in: mode info units
/// of 8×8 and superblocks of 64×64.
pub(super) struct Geometry {
    width: u16,
    height: u16,
    mi_cols: usize,
    mi_rows: usize,
    sb_cols: usize,
    sb_rows: usize,
}

impl Geometry {
    fn new(width: u16, height: u16) -> Self {
        let mi_cols = usize::from(width).div_ceil(8);
        let mi_rows = usize::from(height).div_ceil(8);
        Self { width, height, mi_cols, mi_rows, sb_cols: mi_cols.div_ceil(8), sb_rows: mi_rows.div_ceil(8) }
    }

    /// The fewest tile columns the width allows, as log2.
    fn min_log2_tile_cols(&self) -> u32 {
        (0..).find(|&n| MAX_TILE_WIDTH_B64 << n >= self.sb_cols).expect("a finite width")
    }

    /// The most tile columns the width allows, as log2.
    fn max_log2_tile_cols(&self) -> u32 {
        (1..).find(|&n| self.sb_cols >> n < MIN_TILE_WIDTH_B64).expect("a finite width") - 1
    }

    /// The 8×8 columns of tile `i` of `1 << log2`.
    fn tile_cols(&self, i: usize, log2: u32) -> std::ops::Range<usize> {
        let start = |i: usize| (((i * self.sb_cols) >> log2) * 8).min(self.mi_cols);
        start(i)..start(i + 1)
    }
}

/// The frame as a decoder reconstructs it, three planes at the frame's size
/// rounded up to whole superblocks, so a transform block that reaches past
/// the frame's edge has somewhere to go.
pub(super) struct Reconstruction {
    planes: [Vec<u8>; 3],
    stride: usize,
}

/// One 4:4:4 VP9 stream at one picture size.
pub(crate) struct Encoder {
    geometry: Geometry,
    tile_cols_log2: u32,
    quality: u8,
    contexts: tile::FrameContexts,
    recon: Reconstruction,
}

impl Encoder {
    /// An encoder for a `width`×`height` picture at `quality` (1–100,
    /// clamped), its tile columns chosen as the libvpx encoder's are for
    /// `threads` threads.
    pub(crate) fn new(width: u16, height: u16, quality: u8, threads: usize) -> Result<Self, Error> {
        if width == 0 || height == 0 {
            return Err(Error::Empty(width, height));
        }
        if !(1..=MAX_THREADS).contains(&threads) {
            return Err(Error::Threads(threads));
        }
        let geometry = Geometry::new(width, height);
        let tile_cols_log2 = tile_columns_log2(width, threads).clamp(geometry.min_log2_tile_cols(), geometry.max_log2_tile_cols());
        let stride = geometry.sb_cols * 64;
        let recon = Reconstruction { planes: std::array::from_fn(|_| vec![0; stride * geometry.sb_rows * 64]), stride };
        let contexts = tile::FrameContexts::new(&geometry);
        Ok(Self { geometry, tile_cols_log2, quality: quality.clamp(QUALITY_MIN, QUALITY_MAX), contexts, recon })
    }

    /// Code `picture` as a keyframe and append it to `out`.
    pub(crate) fn keyframe(&mut self, picture: &Picture, out: &mut Vec<u8>) -> Result<(), Error> {
        let g = &self.geometry;
        if picture.size() != (g.width, g.height) || picture.chroma() != Chroma::Full {
            let (pw, ph) = picture.size();
            return Err(Error::Mismatch(g.width, g.height, Chroma::Full.name(), pw, ph, picture.chroma().name()));
        }
        self.contexts.clear();
        let tiles: Vec<Vec<u8>> = (0..1 << self.tile_cols_log2)
            .map(|i| tile::Tile::new(g, g.tile_cols(i, self.tile_cols_log2), &mut self.contexts, &mut self.recon).code())
            .collect();
        let compressed = header::keyframe_compressed_header();
        let uncompressed = header::Keyframe {
            width: g.width,
            height: g.height,
            q_index: tables::QUANTIZER_TO_QINDEX[quality_to_q(self.quality) as usize],
            tile_cols_log2: self.tile_cols_log2,
            min_log2_tile_cols: g.min_log2_tile_cols(),
            max_log2_tile_cols: g.max_log2_tile_cols(),
        };
        out.extend(uncompressed.write(compressed.len()));
        out.extend(compressed);
        // Every tile but the last is preceded by its size.
        let last = tiles.len() - 1;
        for (i, tile) in tiles.iter().enumerate() {
            if i < last {
                out.extend((tile.len() as u32).to_be_bytes());
            }
            out.extend(tile);
        }
        Ok(())
    }

    /// The reconstruction's planes and their stride.
    fn reconstruction(&self) -> (&[Vec<u8>; 3], usize) {
        (&self.recon.planes, self.recon.stride)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Decoder;

    /// A picture of noise, so nothing in it is what the frame decodes to.
    fn noise(w: u16, h: u16) -> Picture {
        let mut state = 0x9e37_79b9_u32;
        let rgb: Vec<u8> = (0..usize::from(w) * usize::from(h) * 3)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let mut picture = Picture::new(w, h, Chroma::Full).expect("a picture");
        picture.read_rgb(&rgb).expect("its own picture");
        picture
    }

    /// Encode a keyframe of `w`×`h` on `threads`, decode it with libvpx, and
    /// hold the decode to the encoder's own reconstruction.
    fn round_trip(w: u16, h: u16, threads: usize) -> Vec<u8> {
        let mut encoder = Encoder::new(w, h, 90, threads).expect("an encoder");
        let picture = noise(w, h);
        let mut frame = Vec::new();
        encoder.keyframe(&picture, &mut frame).expect("a keyframe");
        assert_eq!(crate::frame_header(&frame), Some(crate::FrameHeader { profile: 1, keyframe: true }));
        let mut decoder = Decoder::new(1).expect("a decoder");
        let decoded = decoder.decode(&frame).unwrap_or_else(|e| panic!("{w}x{h} on {threads} threads: {e}"));
        assert_eq!(decoded.size(), (u32::from(w), u32::from(h)));
        assert_eq!(decoded.chroma(), Chroma::Full);
        assert!(decoded.declares_bt601_studio_swing());
        let (recon, stride) = encoder.reconstruction();
        for ((plane, decoded_stride), recon) in decoded.planes().into_iter().zip(decoded.strides()).zip(recon) {
            for y in 0..usize::from(h) {
                let got = &plane[y * decoded_stride..][..usize::from(w)];
                assert_eq!(got, &recon[y * stride..][..usize::from(w)], "{w}x{h}, row {y}");
                assert!(got.iter().all(|&p| p == 128), "{w}x{h}, row {y} is not flat grey");
            }
        }
        frame
    }

    #[test]
    fn a_skipped_keyframe_decodes_flat_grey_at_its_own_size() {
        for (w, h) in [(1, 1), (7, 5), (8, 8), (64, 64), (65, 65), (130, 67), (1440, 900), (1728, 902)] {
            round_trip(w, h, 1);
        }
    }

    #[test]
    fn tile_columns_follow_the_width_and_the_threads() {
        // 3456 wide on 4 threads is two columns of 27 superblocks; on 1, one.
        let two = Encoder::new(3456, 8, 90, 4).expect("an encoder");
        let one = Encoder::new(3456, 8, 90, 1).expect("an encoder");
        assert_eq!((two.tile_cols_log2, one.tile_cols_log2), (1, 0));
        assert_eq!(two.geometry.tile_cols(1, 1), 216..432);
        round_trip(3456, 8, 4);
        // Past 4096 the format itself asks for a second column.
        let wide = Encoder::new(4160, 8, 90, 1).expect("an encoder");
        assert_eq!(wide.tile_cols_log2, 1);
        round_trip(4160, 8, 1);
        // Too narrow for two columns, however many threads.
        assert_eq!(Encoder::new(255, 8, 90, 64).expect("an encoder").tile_cols_log2, 0);
    }

    #[test]
    fn keyframes_follow_each_other_through_one_decoder() {
        let (w, h) = (200, 120);
        let mut encoder = Encoder::new(w, h, 50, 1).expect("an encoder");
        let mut decoder = Decoder::new(1).expect("a decoder");
        for _ in 0..3 {
            let mut frame = Vec::new();
            encoder.keyframe(&noise(w, h), &mut frame).expect("a keyframe");
            decoder.decode(&frame).expect("a decode");
        }
    }

    #[test]
    fn a_picture_that_is_not_the_encoders_is_refused() {
        let mut encoder = Encoder::new(64, 64, 90, 1).expect("an encoder");
        let mut out = Vec::new();
        let subsampled = Picture::new(64, 64, Chroma::Subsampled).expect("a picture");
        assert!(matches!(encoder.keyframe(&subsampled, &mut out), Err(Error::Mismatch(..))));
        assert!(matches!(encoder.keyframe(&noise(64, 32), &mut out), Err(Error::Mismatch(..))));
        assert!(out.is_empty());
        assert!(matches!(Encoder::new(0, 64, 90, 1), Err(Error::Empty(..))));
        assert!(matches!(Encoder::new(64, 64, 90, 0), Err(Error::Threads(0))));
    }
}
