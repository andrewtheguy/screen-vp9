//! A tile's bools and tokens as its blocks are decided, so that the frame's
//! tokens can be counted, the probabilities its header updates chosen from
//! what they count, and everything coded after, at the updated ones
//! (libvpx's tokenize, then pack). A token is kept with the band and context
//! that choose its probabilities, which a decoder works out from the tokens
//! before it, so coding it is the token tree alone.

use super::bool_writer::BoolWriter;
use super::header::{SEGMENT_TREE_PROB, Segmentation};
use super::probs::{Counts, MORE, NODES, coef_slot, counted};
use super::tokens;

/// A token rather than a bool.
const TOKEN: u32 = 1 << 31;
/// A block's segment rather than a bool: in its low bits whether the block
/// is inactive, whether that is what the last map predicts, and the
/// context of the prediction flag. How it is coded is the frame's choice.
const SEGMENT: u32 = 1 << 30;
const PREDICTED: u32 = 2;
const PREDICTION_CONTEXT_SHIFT: u32 = 2;
/// A token whose end of block is coded first, as it is unless the token
/// before was a zero.
const MORE_FIRST: u32 = 1 << 17;
/// The end of block rather than a token.
const END: u32 = 1 << 16;
/// Where a token's band and context are.
const CONTEXT_SHIFT: u32 = 18;

/// Records in the order they are coded: a bool, as the bit under where its
/// probability is in the frame's table (`probs::Probs::table`); or a token,
/// as `TOKEN`, its band and context (`probs::token_context`), whether the
/// end of block is coded first, and the level in its low 16 bits; or an end
/// of block, as `TOKEN`, `END` and its band and context; or a block's
/// segment, as `SEGMENT`.
pub(super) struct Tape {
    records: Vec<u32>,
    /// The tokens recorded, and the bools whose probabilities the frame
    /// updates.
    pub(super) counts: Counts,
}

impl Tape {
    pub(super) fn new() -> Self {
        Self { records: Vec::new(), counts: Counts::new() }
    }

    /// Back to nothing recorded, keeping the room it took.
    pub(super) fn clear(&mut self) {
        self.records.clear();
        self.counts = Counts::new();
    }

    /// Code `bit` at the fixed `probability`.
    #[inline(always)]
    pub(super) fn write(&mut self, bit: bool, probability: u8) {
        self.records.push(u32::from(bit) | u32::from(probability) << 1);
    }

    /// Code `bit` at the probability in `slot` of the frame's table,
    /// counting it where the frame updates that probability.
    pub(super) fn put(&mut self, bit: bool, slot: usize) {
        self.records.push(u32::from(bit) | (slot as u32) << 1);
        if let Some(i) = counted(slot) {
            self.counts.bools[i][usize::from(bit)] += 1;
        }
    }

    /// Code the `token` of `level` at band and context `context`, the end
    /// of block first unless the token before was a zero.
    #[inline(always)]
    pub(super) fn token(&mut self, context: usize, more: bool, level: i32, token: usize) {
        let more = if more { MORE_FIRST } else { 0 };
        self.records.push(TOKEN | (context as u32) << CONTEXT_SHIFT | more | u32::from(level as i16 as u16));
        let counts = &mut self.counts.tokens[context];
        counts[token] += 1;
        counts[MORE] += u32::from(more != 0);
    }

    /// Code an end of block at band and context `context`.
    pub(super) fn end(&mut self, context: usize) {
        self.records.push(TOKEN | END | (context as u32) << CONTEXT_SHIFT);
        let counts = &mut self.counts.tokens[context];
        counts[tokens::EOB_TOKEN] += 1;
        counts[MORE] += 1;
    }

    /// Code a block's segment: `inactive` or not, which the last map
    /// predicts where `predicted`, the prediction flag's context being
    /// `ctx`; whole or as predicted, as the frame chooses once every block
    /// is counted.
    pub(super) fn segment(&mut self, inactive: bool, predicted: bool, ctx: usize) {
        self.records.push(SEGMENT | u32::from(inactive) | if predicted { PREDICTED } else { 0 } | (ctx as u32) << PREDICTION_CONTEXT_SHIFT);
        let counts = &mut self.counts.segments;
        counts.ids[usize::from(inactive)] += 1;
        counts.predicted[ctx][usize::from(predicted)] += 1;
        if !predicted {
            counts.unpredicted[usize::from(inactive)] += 1;
        }
    }

}

/// Everything `tapes` recorded, one after another, coded at the
/// probabilities of `table` (`probs::Probs::table`), the segments as
/// `segmentation` says: predicted or not at its prediction probabilities
/// where it is temporal, and the segment tree where the segment is coded.
pub(super) fn code<'t>(tapes: impl IntoIterator<Item = &'t Tape>, table: &[u8], segmentation: Option<&Segmentation>) -> Vec<u8> {
    let mut w = BoolWriter::new();
    let mut run = w.run();
    for &r in tapes.into_iter().flat_map(|tape| &tape.records) {
        if r & TOKEN == 0 {
            if r & SEGMENT == 0 {
                run.write(r & 1 == 1, table[(r >> 1) as usize]);
                continue;
            }
            let s = segmentation.expect("a frame with segments chose how to code them");
            let predicted = r & PREDICTED != 0;
            if s.temporal {
                run.write(predicted, s.pred[(r >> PREDICTION_CONTEXT_SHIFT & 3) as usize]);
            }
            if !s.temporal || !predicted {
                // Down the tree to `header::INACTIVE_SEGMENT` or segment 0.
                run.write(r & 1 == 1, s.tree);
                run.write(false, SEGMENT_TREE_PROB);
                run.write(false, SEGMENT_TREE_PROB);
            }
            continue;
        }
        let at = coef_slot((r >> CONTEXT_SHIFT & 0x3ff) as usize);
        let p: &[u8; NODES] = table[at..at + NODES].try_into().expect("a probability per node");
        if r & END != 0 {
            run.write(false, p[0]);
        } else {
            tokens::code(&mut run, r & MORE_FIRST != 0, i32::from(r as u16 as i16), p);
        }
    }
    drop(run);
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::super::probs::{Probs, skip_slot, token_context};
    use super::super::tables::CAT6_PROB;
    use super::super::tokens::{EOB_TOKEN, TOKENS};
    use super::*;

    #[test]
    fn bools_and_tokens_are_coded_in_order_and_counted() {
        let mut probs = Probs::default();
        probs.skip[1] = 37;
        let table = probs.table();
        // Band 1 at context 2 of luma intra 4×4 blocks: -3, 0, 70, the end.
        let context = token_context(0, false, false) + 6 + 2;
        let mut tape = Tape::new();
        tape.write(true, 200);
        tape.put(false, skip_slot(1));
        tape.token(context, true, -3, 3);
        tape.token(context, true, 0, 0);
        tape.token(context, false, 70, 10);
        tape.end(context);
        // Two tapes coded one after the other are one stream.
        let mut after = Tape::new();
        after.put(true, skip_slot(1));
        let data = code([&tape, &after], &table, None);
        let mut r = BoolReader::new(&data);
        let at = coef_slot(context);
        let p = &table[at..at + NODES];
        let mut read = |bits: &[(bool, u8)]| bits.iter().all(|&(bit, p)| r.read(p) == bit);
        assert!(read(&[(true, 200), (false, 37)]), "the skip at the table's probability");
        // More, not a zero, not a one, a three, negative.
        assert!(read(&[(true, p[0]), (true, p[1]), (true, p[2]), (false, p[3]), (true, p[4]), (false, p[5]), (true, 128)]), "-3");
        assert!(read(&[(true, p[0]), (false, p[1])]), "0");
        // After the zero no end of block's node; then category 6 and 70 less
        // its 67 in fourteen bits.
        let extra: Vec<(bool, u8)> = CAT6_PROB.iter().enumerate().map(|(i, &q)| (3 >> (13 - i) & 1 == 1, q)).collect();
        assert!(read(&[(true, p[1]), (true, p[2]), (true, p[3]), (true, p[6]), (true, p[8]), (true, p[10])]) && read(&extra) && read(&[(false, 128)]), "70");
        assert!(read(&[(false, p[0]), (true, 37)]), "the end, then the skip");
        assert_eq!(tape.counts.tokens[context][..TOKENS], [1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1]);
        assert_eq!(tape.counts.tokens[context][MORE], 3, "the end of block's node before -3, 0 and the end");
        assert_eq!(tape.counts.tokens[context][EOB_TOKEN], 1);
        assert_eq!(tape.counts.bools[counted(skip_slot(1)).expect("a counted slot")], [1, 0]);
    }
}
