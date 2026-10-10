//! A frame's two headers: the uncompressed one, plain bits, and the
//! compressed one, bool coded. The fields are written in the order the VP9
//! specification reads them (§6.2, §6.3), each at the one value the subset
//! the encoder codes allows, but for the probability updates.

use super::bool_writer::{self, BoolWriter};
use super::probs::{Counts, DIFF_UPDATE_PROB, Probs, SegmentCounts, binary_prob};

/// The most a frame's compressed header can say it holds: its size is a
/// 16-bit field.
const MAX_COMPRESSED_HEADER: usize = u16::MAX as usize;

/// `color_space` for BT.601.
const CS_BT_601: u32 = 1;

/// `tx_mode` ALLOW_32X32, which with `tx_mode_select` after it is
/// TX_MODE_SELECT: each block says which transform size it takes, up to the
/// largest that fits it.
const ALLOW_32X32: u32 = 3;

/// The probability an update flag for a motion vector probability is coded
/// at.
const MV_UPDATE_PROB: u8 = 252;

/// The reference slot an inter frame reads and refreshes: the one the
/// keyframe filled with the rest.
const LAST_SLOT: u32 = 0;

/// `interp_filter`'s literal for EIGHTTAP, the frame's one filter, which
/// no block uses: every vector is of whole pixels.
const EIGHTTAP: u32 = 1;

/// The segment of a block an inter frame leaves as the previous frame has
/// it: the previous frame at the zero vector, no residual, and not loop
/// filtered, as libvpx codes the blocks outside its active map. The segment
/// tree is three nodes deep for every segment, and segment 4 differs from
/// segment 0, the active one, at the first node alone: the frame's
/// probability goes there, and the other six stay at 255. Its features: the
/// loop filter level `-MAX_LOOP_FILTER`, which is 0 whatever the frame's;
/// the reference LAST; and skip.
pub(super) const INACTIVE_SEGMENT: u32 = 4;
/// The probability of every node of the segment tree but its first.
pub(super) const SEGMENT_TREE_PROB: u8 = 255;
const SEGMENTS: usize = 8;
const ALT_LF_BITS: u32 = 6;
const MAX_LOOP_FILTER: u32 = 63;
const REF_FRAME_BITS: u32 = 2;
const LAST_FRAME: u32 = 1;
/// What a probability costs the header when it is not 255: its flag and
/// eight bits.
const PROB_BITS: u32 = 9;

/// An inter frame's segmentation: its blocks in the active segment or the
/// inactive one, each saying which, whole or as the map of the frame before
/// predicts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Segmentation {
    /// Whether each block first says whether its segment is the last map's
    /// (libvpx's `temporal_update`), and codes it only where it is not.
    pub(super) temporal: bool,
    /// The probability of a coded segment being active, 255 for one in 256.
    pub(super) tree: u8,
    /// Per context, the probability of a block's segment not being the last
    /// map's; unused unless `temporal`.
    pub(super) pred: [u8; 3],
    /// Whether the inactive segment's features are written, which the first
    /// segmented frame after a keyframe does, and the ones after it leave
    /// in force.
    pub(super) features: bool,
}

impl Segmentation {
    /// The cheaper way of coding the segments `counts` counted, at the
    /// probabilities they measure, with the header's own bits for the
    /// probabilities weighed in (libvpx's `vp9_choose_segmap_coding_method`
    /// for two segments).
    pub(super) fn choose(counts: &SegmentCounts, features: bool) -> Self {
        let branch = |ct: [u32; 2], p: u8| u64::from(ct[0]) * u64::from(bool_writer::cost(false, p)) + u64::from(ct[1]) * u64::from(bool_writer::cost(true, p));
        let header = |p: u8| if p == 255 { 0 } else { u64::from(PROB_BITS) << 8 };
        let whole = binary_prob(counts.ids);
        let whole_cost = branch(counts.ids, whole) + header(whole);
        let pred: [u8; 3] = std::array::from_fn(|ctx| binary_prob(counts.predicted[ctx]));
        let tree = binary_prob(counts.unpredicted);
        let temporal_cost = pred.iter().zip(counts.predicted).map(|(&p, ct)| branch(ct, p) + header(p)).sum::<u64>() + branch(counts.unpredicted, tree) + header(tree);
        if temporal_cost < whole_cost { Self { temporal: true, tree, pred, features } } else { Self { temporal: false, tree: whole, pred: [255; 3], features } }
    }
}

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

/// What a frame's uncompressed header says.
pub(super) struct Uncompressed {
    /// A keyframe, or an inter frame reading the frame before it.
    pub(super) keyframe: bool,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) q_index: u8,
    /// The loop filter's level, 0 for none.
    pub(super) filter_level: u8,
    pub(super) tile_cols_log2: u32,
    pub(super) min_log2_tile_cols: u32,
    pub(super) max_log2_tile_cols: u32,
    /// An inter frame's segments; none on a keyframe.
    pub(super) segmentation: Option<Segmentation>,
}

impl Uncompressed {
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
        w.flag(!self.keyframe); // frame_type
        w.flag(true); // show_frame
        w.flag(false); // error_resilient_mode
        if self.keyframe {
            self.write_keyframe(&mut w);
        } else {
            self.write_inter(&mut w);
        }
        self.write_tail(&mut w, compressed_size);
        w.bytes
    }

    /// A keyframe's: the colour and the size, every reference slot refreshed.
    fn write_keyframe(&self, w: &mut BitWriter) {
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
    }

    /// An inter frame's: one slot refreshed and read through all three
    /// references, the size the reference has.
    fn write_inter(&self, w: &mut BitWriter) {
        // intra_only is implied clear by show_frame.
        w.put(0, 2); // reset_frame_context
        w.put(1 << LAST_SLOT, 8); // refresh_frame_flags
        for _ in 0..3 {
            w.put(LAST_SLOT, 3); // ref_frame_idx
            // Every sign bias alike, which also rules out compound prediction.
            w.flag(false); // ref_frame_sign_bias
        }
        w.flag(true); // found_ref: the size of LAST
        w.flag(false); // render_and_frame_size_different
        w.flag(false); // allow_high_precision_mv
        w.flag(false); // is_filter_switchable
        w.put(EIGHTTAP, 2); // raw_interpolation_filter
    }

    /// From `refresh_frame_context` on, which every frame writes alike.
    fn write_tail(&self, w: &mut BitWriter, compressed_size: usize) {
        // The decoder keeps the probabilities a frame's header updated for
        // the next, and does not adapt them to what the frame coded.
        w.flag(true); // refresh_frame_context
        w.flag(true); // frame_parallel_decoding_mode
        w.put(0, 2); // frame_context_idx
        // loop_filter_params: the deltas enabled at the values a decoder
        // starts each keyframe with, an intra block's level one more.
        w.put(u32::from(self.filter_level), 6); // loop_filter_level
        w.put(0, 3); // loop_filter_sharpness
        w.flag(true); // loop_filter_delta_enabled
        w.flag(false); // loop_filter_delta_update
        // quantization_params: one q index for every coefficient.
        w.put(u32::from(self.q_index), 8);
        w.flag(false); // delta_coded, luma DC
        w.flag(false); // delta_coded, chroma DC
        w.flag(false); // delta_coded, chroma AC
        // segmentation_params.
        match &self.segmentation {
            None => w.flag(false), // segmentation_enabled
            Some(s) => {
                w.flag(true); // segmentation_enabled
                // The map: the tree's first node at the frame's probability,
                // the rest at 255, which is not written; and, predicted,
                // the prediction flag's probabilities.
                w.flag(true); // segmentation_update_map
                let prob = |w: &mut BitWriter, p: u8| {
                    w.flag(p != 255);
                    if p != 255 {
                        w.put(u32::from(p), 8);
                    }
                };
                prob(w, s.tree); // segmentation_tree_probs[0]
                for _ in 1..7 {
                    w.flag(false);
                }
                w.flag(s.temporal); // segmentation_temporal_update
                if s.temporal {
                    for p in s.pred {
                        prob(w, p); // segmentation_pred_prob
                    }
                }
                w.flag(s.features); // segmentation_update_data
                if s.features {
                    w.flag(false); // segmentation_abs_or_delta_update: deltas
                    for segment in 0..SEGMENTS as u32 {
                        let inactive = segment == INACTIVE_SEGMENT;
                        w.flag(false); // SEG_LVL_ALT_Q
                        w.flag(inactive); // SEG_LVL_ALT_LF
                        if inactive {
                            w.put(MAX_LOOP_FILTER, ALT_LF_BITS);
                            w.flag(true); // negative
                        }
                        w.flag(inactive); // SEG_LVL_REF_FRAME
                        if inactive {
                            w.put(LAST_FRAME, REF_FRAME_BITS);
                        }
                        w.flag(inactive); // SEG_LVL_SKIP
                    }
                }
            }
        }
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

/// The compressed header: the transform mode, each block saying its size;
/// the updates to the transform size, coefficient and skip probabilities
/// `probs` that the frame's bools, `counts`, call for, made to `probs`; and
/// no update to any other probability, of which an inter frame has more.
pub(super) fn compressed(keyframe: bool, probs: &mut Probs, counts: &Counts) -> Vec<u8> {
    let mut w = BoolWriter::new();
    // tx_mode: TX_MODE_SELECT, and the transform size probabilities.
    w.literal(ALLOW_32X32, 2);
    w.bit(true);
    probs.update_tx(&mut w, counts);
    // The coefficients of each transform size up to 32×32, then skip.
    probs.update_coefs(&mut w, counts, 3);
    probs.update_skip(&mut w, counts);
    if !keyframe {
        // The inter modes (7 contexts of 3), intra or inter (4), the single
        // reference (5 of 2), the luma intra modes (4 of 9) and the
        // partitions (16 of 3). No switchable filter and no compound
        // reference, so nothing for either.
        for _ in 0..7 * 3 + 4 + 5 * 2 + 4 * 9 + 16 * 3 {
            w.write(false, DIFF_UPDATE_PROB);
        }
        // The motion vectors: joints (3); per component sign (1), classes
        // (10), class 0 (1) and bits (10); per component class 0's fractions
        // (2 of 3) and the fractions (3). No high precision.
        for _ in 0..3 + 2 * (1 + 10 + 1 + 10) + 2 * (2 * 3 + 3) {
            w.write(false, MV_UPDATE_PROB);
        }
    }
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::super::probs::tests::read_update;
    use super::super::probs::{skip_slot, tx_slot};
    use super::super::tables::{COEF_PROBS, SKIP_PROBS, TX_PROBS_8X8, TX_PROBS_16X16, TX_PROBS_32X32};
    use super::super::tape::Tape;
    use super::super::tokens;
    use super::super::transform::TxType;
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
        let header = Uncompressed { keyframe: true, width: 1440, height: 900, q_index: 32, filter_level: 10, tile_cols_log2: 0, min_log2_tile_cols: 0, max_log2_tile_cols: 2, segmentation: None };
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
        assert_eq!(field(72, 2), 0b11, "refreshed, frame parallel");
        assert_eq!(field(74, 2), 0, "context 0");
        assert_eq!(field(76, 6), 10, "loop filter level");
        assert_eq!(field(82, 5), 0b00010, "sharpness 0, deltas on and not updated");
        assert_eq!(field(87, 8), 32);
        // Three deltas and segmentation off, a 0 that stops the tile columns
        // at one, one tile row, then the size: 117 bits in 15 bytes.
        assert_eq!(field(95, 6), 0);
        assert_eq!(field(101, 16), 7);
        assert_eq!(bytes.len(), 15);
    }

    #[test]
    fn the_inter_header_reads_last_and_refreshes_it() {
        let header = Uncompressed { keyframe: false, width: 1440, height: 900, q_index: 32, filter_level: 10, tile_cols_log2: 0, min_log2_tile_cols: 0, max_log2_tile_cols: 2, segmentation: None };
        let bytes = header.write(9);
        let crate::FrameHeader { profile, keyframe } = crate::frame_header(&bytes).expect("a VP9 frame");
        assert_eq!((profile, keyframe), (1, false));
        let bit = |i: usize| bytes[i / 8] >> (7 - i % 8) & 1;
        let field = |at: usize, n: usize| (at..at + n).fold(0u32, |v, i| v << 1 | u32::from(bit(i)));
        assert_eq!(field(4, 4), 0b0110, "shown, not error resilient");
        assert_eq!(field(8, 2), 0, "no context reset");
        assert_eq!(field(10, 8), 1, "slot 0 refreshed");
        assert_eq!(field(18, 12), 0, "every reference slot 0, no sign bias");
        assert_eq!(field(30, 2), 0b10, "the size of LAST, no render size");
        assert_eq!(field(32, 4), 0b0001, "no high precision, EIGHTTAP");
        assert_eq!(field(36, 2), 0b11, "refreshed, frame parallel");
        assert_eq!(field(38, 2), 0, "context 0");
        assert_eq!(field(40, 11), 10 << 5 | 0b00010, "loop filter level, sharpness, deltas");
        assert_eq!(field(51, 8), 32);
        assert_eq!(field(59, 6), 0);
        assert_eq!(field(65, 16), 9);
        assert_eq!(bytes.len(), 11);
    }

    /// The segmented inter header, read bit by bit from the quantizer on: the
    /// map coded whole at the frame's probability, and the inactive
    /// segment's three features, written once and then left in force; or
    /// predicted, with the prediction's probabilities.
    #[test]
    fn the_segmented_header_codes_the_map_and_the_inactive_segments_features() {
        let header = |tree, features| Uncompressed { keyframe: false, width: 1440, height: 900, q_index: 32, filter_level: 10, tile_cols_log2: 0, min_log2_tile_cols: 0, max_log2_tile_cols: 2, segmentation: Some(Segmentation { temporal: false, tree, pred: [255; 3], features }) };
        let bytes = header(200, true).write(9);
        let bit = |i: usize| bytes[i / 8] >> (7 - i % 8) & 1;
        let field = |at: usize, n: usize| (at..at + n).fold(0u32, |v, i| v << 1 | u32::from(bit(i)));
        assert_eq!(field(51, 8), 32, "the q index where it was");
        assert_eq!(field(59, 3), 0, "no deltas");
        assert_eq!(field(62, 3), 0b111, "segmentation enabled, the map updated, its first probability coded");
        assert_eq!(field(65, 8), 200);
        assert_eq!(field(73, 6), 0, "the other six probabilities 255");
        assert_eq!(field(79, 2), 0b01, "not temporal, the features updated");
        assert_eq!(field(81, 1), 0, "deltas");
        // Four flags for each of segments 0 to 3, then segment 4's: no ALT_Q,
        // ALT_LF -63, REF_FRAME 1, SKIP; then three segments of four.
        assert_eq!(field(82, 16), 0);
        assert_eq!(field(98, 1), 0);
        assert_eq!(field(99, 8), 0xff, "ALT_LF on, 63, negative");
        assert_eq!(field(107, 3), 0b101, "REF_FRAME on, LAST");
        assert_eq!(field(110, 1), 1);
        assert_eq!(field(111, 12), 0);
        // One tile column, one tile row, and the size.
        assert_eq!(field(123, 2), 0);
        assert_eq!(field(125, 16), 9);
        assert_eq!(bytes.len(), 18);

        let bytes = header(255, false).write(9);
        let bit = |i: usize| bytes[i / 8] >> (7 - i % 8) & 1;
        let field = |at: usize, n: usize| (at..at + n).fold(0u32, |v, i| v << 1 | u32::from(bit(i)));
        assert_eq!(field(62, 2), 0b11);
        assert_eq!(field(64, 9), 0, "no probability coded, not temporal, no features");
        assert_eq!(field(73, 2), 0);
        assert_eq!(field(75, 16), 9);
        assert_eq!(bytes.len(), 12);

        let mut predicted = header(255, false);
        predicted.segmentation = Some(Segmentation { temporal: true, tree: 20, pred: [255, 30, 40], features: false });
        let bytes = predicted.write(9);
        let bit = |i: usize| bytes[i / 8] >> (7 - i % 8) & 1;
        let field = |at: usize, n: usize| (at..at + n).fold(0u32, |v, i| v << 1 | u32::from(bit(i)));
        assert_eq!(field(62, 3), 0b111);
        assert_eq!(field(65, 8), 20);
        assert_eq!(field(73, 7), 1, "six at 255, then temporal");
        assert_eq!(field(80, 1), 0, "the first prediction probability 255");
        assert_eq!(field(81, 9), 1 << 8 | 30);
        assert_eq!(field(90, 9), 1 << 8 | 40);
        assert_eq!(field(99, 1), 0, "no features");
        assert_eq!(field(100, 2), 0);
        assert_eq!(field(102, 16), 9);
        assert_eq!(bytes.len(), 15);
    }

    /// A map that is mostly the last frame's is coded against it, and one
    /// that is not is coded whole, each at the probabilities its counts
    /// measure.
    #[test]
    fn the_map_is_coded_the_cheaper_way() {
        let mut counts = SegmentCounts { ids: [600, 400], predicted: [[5, 95], [10, 490], [15, 385]], unpredicted: [20, 10] };
        let s = Segmentation::choose(&counts, true);
        assert!(s.temporal && s.features);
        assert_eq!(s.pred, [13, 5, 10]);
        assert_eq!(s.tree, 171);
        counts.predicted = [[50, 50], [250, 250], [200, 200]];
        counts.unpredicted = [300, 200];
        let s = Segmentation::choose(&counts, false);
        assert_eq!(s, Segmentation { temporal: false, tree: 154, pred: [255; 3], features: false });
        // Every block active: nothing predicted says anything cheaper than
        // a map of one probability the header leaves out.
        let all = SegmentCounts { ids: [1000, 0], predicted: [[0, 1000], [0, 0], [0, 0]], unpredicted: [0; 2] };
        let s = Segmentation::choose(&all, false);
        assert_eq!((s.temporal, s.tree), (false, 255));
    }

    /// The transform size probabilities' part of a compressed header as a
    /// decoder reads it (libvpx's `tx_mode_probs`), into `probs`.
    fn read_tx(r: &mut BoolReader, probs: &mut Probs) {
        for p in probs.tx8.iter_mut().flatten().chain(probs.tx16.iter_mut().flatten()).chain(probs.tx32.iter_mut().flatten()) {
            *p = read_update(r, *p);
        }
    }

    /// The coefficient probabilities' part of a compressed header as a
    /// decoder reads it (libvpx's `read_coef_probs`), into `probs`.
    fn read_coefs(r: &mut BoolReader, probs: &mut Probs) {
        for tx in 0..4 {
            if !r.read(128) {
                continue;
            }
            for (band, contexts) in probs.coef[tx].iter_mut().flatten().flat_map(|kind| kind.iter_mut().enumerate()) {
                for p in contexts[..if band == 0 { 3 } else { 6 }].iter_mut().flatten() {
                    *p = read_update(r, *p);
                }
            }
        }
    }

    /// A compressed header up to the inter frame's modes, as a decoder reads
    /// it into `probs`: the transform mode, then the updates.
    fn read<'a>(bytes: &'a [u8], probs: &mut Probs) -> BoolReader<'a> {
        let mut r = BoolReader::new(bytes);
        assert_eq!(r.literal(2), ALLOW_32X32);
        assert!(r.read(128), "tx_mode_select");
        read_tx(&mut r, probs);
        read_coefs(&mut r, probs);
        for p in &mut probs.skip {
            *p = read_update(&mut r, *p);
        }
        r
    }

    #[test]
    fn the_compressed_header_selects_transform_sizes_and_updates_nothing_without_bools() {
        for keyframe in [true, false] {
            let mut probs = Probs::default();
            let bytes = compressed(keyframe, &mut probs, &Counts::new());
            let mut read = Probs::default();
            let mut r = super::tests::read(&bytes, &mut read);
            assert_eq!((read.tx8, read.tx16, read.tx32, read.skip, read.coef), (probs.tx8, probs.tx16, probs.tx32, probs.skip, probs.coef));
            assert_eq!((probs.tx8, probs.tx16, probs.tx32, probs.skip, probs.coef), (TX_PROBS_8X8, TX_PROBS_16X16, TX_PROBS_32X32, SKIP_PROBS, COEF_PROBS));
            if !keyframe {
                assert!((0..119).all(|_| !r.read(DIFF_UPDATE_PROB)), "no mode update");
                assert!((0..65).all(|_| !r.read(MV_UPDATE_PROB)), "no vector update");
            }
        }
    }

    #[test]
    fn coefficient_updates_follow_the_tokens_and_read_back() {
        // Luma intra 8×8 blocks of a lone DC of 3, far from what the defaults
        // expect, and nothing else.
        let mut tape = Tape::new();
        let mut levels = [0; 64];
        levels[0] = 3;
        let mut energy = [0; 1024];
        for _ in 0..2000 {
            tokens::write(&mut tape, tokens::Kind { tx: 1, tx_type: TxType::DctDct, chroma: false, inter: false }, 0, &levels, 1, &mut energy);
        }
        let before = Probs::default();
        let mut probs = before.clone();
        let bytes = compressed(false, &mut probs, &tape.counts);
        let mut read = before.clone();
        super::tests::read(&bytes, &mut read);
        assert_eq!(read.coef, probs.coef, "what a decoder reads is what the encoder codes at");
        assert_eq!([probs.coef[0], probs.coef[2], probs.coef[3]], [before.coef[0], before.coef[2], before.coef[3]], "only 8x8 updated");
        let (now, then) = (probs.coef[1][0][0], before.coef[1][0][0]);
        assert!(now[0][0][1] < then[0][0][1] && now[0][0][2] < then[0][0][2], "the DC more often not a zero, and more often more than one");
        // After a DC of 3 the next position is in band 1 at context 3.
        assert!(now[1][3][0] > then[1][3][0], "the block more often ending after it");
    }

    #[test]
    fn transform_size_and_skip_updates_follow_the_bools_and_read_back() {
        // 32×32 blocks at context 1 nearly all 4×4, and blocks beside two
        // skipped ones nearly all skipped too.
        let mut tape = Tape::new();
        for i in 0..3000 {
            tape.put(i % 50 == 0, tx_slot(3, 1));
            tape.put(i % 40 != 0, skip_slot(2));
        }
        let before = Probs::default();
        let mut probs = before.clone();
        let bytes = compressed(true, &mut probs, &tape.counts);
        let mut read = before.clone();
        super::tests::read(&bytes, &mut read);
        assert_eq!((read.tx8, read.tx16, read.tx32, read.skip), (probs.tx8, probs.tx16, probs.tx32, probs.skip));
        assert!(probs.tx32[1][0] > before.tx32[1][0], "4×4 likelier");
        assert_eq!((probs.tx32[0], &probs.tx32[1][1..], probs.tx8, probs.tx16), (before.tx32[0], &before.tx32[1][1..], before.tx8, before.tx16));
        assert!(probs.skip[2] < before.skip[2], "skips likelier");
        assert_eq!(probs.skip[..2], before.skip[..2]);
        assert_eq!(probs.coef, before.coef);
    }
}
