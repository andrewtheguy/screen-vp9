//! A frame's two headers: the uncompressed one, plain bits, and the
//! compressed one, bool coded. The fields are written in the order the VP9
//! specification reads them (§6.2, §6.3), each at the one value the subset
//! the encoder codes allows.

use super::bool_writer::BoolWriter;

/// The most a frame's compressed header can say it holds: its size is a
/// 16-bit field.
const MAX_COMPRESSED_HEADER: usize = u16::MAX as usize;

/// `color_space` for BT.601.
const CS_BT_601: u32 = 1;

/// `tx_mode` ALLOW_32X32: every block takes the largest transform that fits
/// it, up to 32×32, and no block says which.
const ALLOW_32X32: u32 = 3;

/// The probability an update flag for a probability is coded at.
const DIFF_UPDATE_PROB: u8 = 252;

/// Bits written most significant first.
#[derive(Default)]
pub(super) struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}

impl BitWriter {
    fn put(&mut self, value: u32, bits: u32) {
        for i in (0..bits).rev() {
            if self.bits.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = (value >> i & 1) as u8;
            *self.bytes.last_mut().expect("a byte was pushed") |= bit << (7 - self.bits % 8);
            self.bits += 1;
        }
    }

    fn flag(&mut self, flag: bool) {
        self.put(u32::from(flag), 1);
    }
}

/// What the uncompressed header of a keyframe says.
pub(super) struct Keyframe {
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) q_index: u8,
    pub(super) tile_cols_log2: u32,
    pub(super) min_log2_tile_cols: u32,
    pub(super) max_log2_tile_cols: u32,
}

impl Keyframe {
    /// The uncompressed header, ending with the size of the compressed header
    /// that follows it.
    pub(super) fn write(&self, compressed_size: usize) -> Vec<u8> {
        assert!(compressed_size <= MAX_COMPRESSED_HEADER, "a compressed header of {compressed_size} bytes");
        let mut w = BitWriter::default();
        w.put(2, 2); // frame_marker
        // Profile 1: the low bit set, the high bit clear.
        w.put(1, 1);
        w.put(0, 1);
        w.flag(false); // show_existing_frame
        w.flag(false); // frame_type: a keyframe
        w.flag(true); // show_frame
        w.flag(false); // error_resilient_mode
        w.put(0x49_83_42, 24); // frame_sync_code
        // color_config: eight bits implied by profile 1, BT.601 at studio
        // swing, no chroma subsampling.
        w.put(CS_BT_601, 3);
        w.flag(false); // color_range: studio swing
        w.flag(false); // subsampling_x
        w.flag(false); // subsampling_y
        w.flag(false); // reserved_zero
        w.put(u32::from(self.width) - 1, 16);
        w.put(u32::from(self.height) - 1, 16);
        w.flag(false); // render_and_frame_size_different
        self.write_tail(&mut w, compressed_size);
        w.bytes
    }

    /// From `refresh_frame_context` on, which every frame writes alike.
    fn write_tail(&self, w: &mut BitWriter, compressed_size: usize) {
        // Probabilities are the defaults on every frame: the decoder neither
        // adapts them after a frame nor keeps what one did.
        w.flag(false); // refresh_frame_context
        w.flag(true); // frame_parallel_decoding_mode
        w.put(0, 2); // frame_context_idx
        // loop_filter_params: level 0, so no filter runs.
        w.put(0, 6); // loop_filter_level
        w.put(0, 3); // loop_filter_sharpness
        w.flag(false); // loop_filter_delta_enabled
        // quantization_params: one q index for every coefficient.
        w.put(u32::from(self.q_index), 8);
        w.flag(false); // delta_coded, luma DC
        w.flag(false); // delta_coded, chroma DC
        w.flag(false); // delta_coded, chroma AC
        w.flag(false); // segmentation_enabled
        // tile_info: increments from the fewest columns, a zero ending them
        // short of the most, and one tile row.
        for _ in self.min_log2_tile_cols..self.tile_cols_log2 {
            w.flag(true);
        }
        if self.tile_cols_log2 < self.max_log2_tile_cols {
            w.flag(false);
        }
        w.flag(false); // tile_rows_log2
        w.put(compressed_size as u32, 16); // header_size_in_bytes
    }
}

/// The compressed header of a keyframe: the transform mode and no update to
/// any default probability.
pub(super) fn keyframe_compressed_header() -> Vec<u8> {
    let mut w = BoolWriter::new();
    // tx_mode: ALLOW_32X32 and, after it, tx_mode_select clear.
    w.literal(ALLOW_32X32, 2);
    w.bit(false);
    // update_probs for the coefficients of each transform size up to 32×32.
    for _ in 0..4 {
        w.bit(false);
    }
    // No update to any of the three skip probabilities.
    for _ in 0..3 {
        w.write(false, DIFF_UPDATE_PROB);
    }
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::*;

    #[test]
    fn bits_go_most_significant_first() {
        let mut w = BitWriter::default();
        w.put(0b101, 3);
        w.put(0x1ff, 9);
        assert_eq!(w.bytes, [0b1011_1111, 0b1111_0000]);
    }

    #[test]
    fn the_keyframe_header_reads_as_profile_1_at_bt601_studio_swing() {
        let header = Keyframe { width: 1440, height: 900, q_index: 32, tile_cols_log2: 0, min_log2_tile_cols: 0, max_log2_tile_cols: 2 };
        let bytes = header.write(7);
        let crate::FrameHeader { profile, keyframe } = crate::frame_header(&bytes).expect("a VP9 frame");
        assert_eq!((profile, keyframe), (1, true));
        let bit = |i: usize| bytes[i / 8] >> (7 - i % 8) & 1;
        let field = |at: usize, n: usize| (at..at + n).fold(0u32, |v, i| v << 1 | u32::from(bit(i)));
        // After marker, profile, three flags and the sync code: the colour.
        assert_eq!(field(8, 24), 0x49_83_42);
        assert_eq!(field(32, 3), CS_BT_601);
        assert_eq!(field(35, 4), 0, "studio swing, no subsampling");
        assert_eq!(field(39, 16), 1439);
        assert_eq!(field(55, 16), 899);
        // Render size, the two context flags, the context index, the loop
        // filter, then the q index.
        assert_eq!(field(71, 1), 0);
        assert_eq!(field(72, 2), 0b01, "no refresh, frame parallel");
        assert_eq!(field(74, 12), 0, "context 0, loop filter level, sharpness and deltas off");
        assert_eq!(field(86, 8), 32);
        // Three deltas and segmentation off, a 0 that stops the tile columns
        // at one, one tile row, then the size: 116 bits in 15 bytes.
        assert_eq!(field(94, 6), 0);
        assert_eq!(field(100, 16), 7);
        assert_eq!(bytes.len(), 15);
    }

    #[test]
    fn the_compressed_header_asks_for_32x32_and_updates_nothing() {
        let bytes = keyframe_compressed_header();
        let mut r = BoolReader::new(&bytes);
        assert_eq!(r.literal(2), ALLOW_32X32);
        assert!(!r.read(128), "tx_mode_select");
        assert!((0..4).all(|_| !r.read(128)), "no coefficient update");
        assert!((0..3).all(|_| !r.read(DIFF_UPDATE_PROB)), "no skip update");
    }
}
