//! The coefficient probabilities a frame's tokens are coded at, kept from
//! frame to frame as a decoder keeps them, and the updates to them a frame's
//! compressed header carries, each chosen from the frame's own tokens where
//! it saves more than it costs (libvpx's `update_coef_probs`, its two-loop
//! form).

use super::bool_writer::{self, BoolWriter};
use super::tables::{COEF_PROBS, CoefProbs, PARETO8_FULL, SKIP_PROBS, TX_PROBS_8X8, TX_PROBS_16X16, TX_PROBS_32X32, UPDATE_INDEX};
use super::tokens::{EOB_TOKEN, TOKENS};

/// The probability an update flag for a probability is coded at.
pub(super) const DIFF_UPDATE_PROB: u8 = 252;

/// The third node, whose probability gives the eight after it.
const PIVOT_NODE: usize = 2;

/// An update has to save more than this, in 256ths of a bit, besides its own
/// flag, to be looked for at all (libvpx's `MIN_DELP_BITS`).
const LEAST_UPDATE: i64 = 5 << 8;

/// The entries of a frame's table that hold each probability as itself.
const FIXED: usize = 256;
/// Where in a frame's table, after `FIXED`, the transform size
/// probabilities of a block whose largest transform is 8×8, 16×16 and
/// 32×32 start: per context, a probability per node.
const TX_SLOTS: [usize; 3] = [0, 2, 6];
/// Where the skip probabilities start, after `FIXED`.
const SKIP_SLOTS: usize = 12;
/// The entries after `FIXED` for the bools other than tokens that a frame
/// updates the probabilities of.
const MODE_SLOTS: usize = SKIP_SLOTS + 3;

/// The slot of the first node of the transform size tree of a block whose
/// largest transform is `max` (1 for 8×8 to 3 for 32×32) at `ctx`.
pub(super) fn tx_slot(max: usize, ctx: usize) -> usize {
    FIXED + TX_SLOTS[max - 1] + ctx * max
}

/// The slot of the skip probability at `ctx`.
pub(super) fn skip_slot(ctx: usize) -> usize {
    FIXED + SKIP_SLOTS + ctx
}

/// Where a bool at `slot` is counted, if the frame updates the probability
/// there from its bools.
#[inline(always)]
pub(super) fn counted(slot: usize) -> Option<usize> {
    (FIXED..FIXED + MODE_SLOTS).contains(&slot).then(|| slot - FIXED)
}
/// The nodes of the token tree.
pub(super) const NODES: usize = 11;
/// The bands and contexts of every kind of transform block: per size,
/// chroma or not and inter or not, six bands of six contexts.
pub(super) const CONTEXTS: usize = 4 * 2 * 2 * 6 * 6;

/// Where in `Counts` the times the end of block was coded are, after the
/// tokens.
pub(super) const MORE: usize = TOKENS;

/// The first band and context of transform blocks of size `tx`, chroma or
/// not and inter or not; band and context then count from it, six contexts
/// to a band.
pub(super) fn token_context(tx: usize, chroma: bool, inter: bool) -> usize {
    ((tx * 2 + usize::from(chroma)) * 2 + usize::from(inter)) * 36
}

/// Where the token tree's nodes at band and context `context` are in a
/// frame's table.
#[inline(always)]
pub(super) fn coef_slot(context: usize) -> usize {
    FIXED + MODE_SLOTS + context * NODES
}

/// A frame's tokens, per band and context how many of each and how many
/// times the end of block was coded, and its other bools whose
/// probabilities it updates, per slot after `FIXED` how many were false and
/// how many true.
pub(super) struct Counts {
    pub(super) tokens: Box<[[u32; TOKENS + 1]; CONTEXTS]>,
    pub(super) bools: [[u32; 2]; MODE_SLOTS],
}

impl Counts {
    pub(super) fn new() -> Self {
        Self { tokens: Box::new([[0; TOKENS + 1]; CONTEXTS]), bools: [[0; 2]; MODE_SLOTS] }
    }

    /// Add `other`'s counts to these.
    pub(super) fn add(&mut self, other: &Self) {
        for (mine, theirs) in self.tokens.iter_mut().flatten().zip(other.tokens.iter().flatten()) {
            *mine += theirs;
        }
        for (mine, theirs) in self.bools.iter_mut().flatten().zip(other.bools.iter().flatten()) {
            *mine += theirs;
        }
    }

    /// The counts down each of the token tree's nodes at a band and context
    /// of a kind of transform block (libvpx's
    /// `vp9_tree_probs_from_distribution` over `vp9_coef_tree`, the first
    /// node's from the times it was coded).
    fn branches(&self, tx: usize, plane: usize, reference: usize, band: usize, ctx: usize) -> [[u32; 2]; NODES] {
        let t = &self.tokens[token_context(tx, plane == 1, reference == 1) + band * 6 + ctx];
        let sum = |r: std::ops::Range<usize>| t[r].iter().sum::<u32>();
        [
            [t[EOB_TOKEN], t[MORE] - t[EOB_TOKEN]],
            [t[0], sum(1..11)],
            [t[1], sum(2..11)],
            [sum(2..5), sum(5..11)],
            [t[2], t[3] + t[4]],
            [t[3], t[4]],
            [t[5] + t[6], sum(7..11)],
            [t[5], t[6]],
            [t[7] + t[8], t[9] + t[10]],
            [t[7], t[8]],
            [t[9], t[10]],
        ]
    }
}

/// The probabilities of a frame context that the encoder updates.
#[derive(Clone)]
pub(super) struct Probs {
    /// The transform size of a block whose largest is 8×8, 16×16 and 32×32,
    /// `[context][node]`.
    pub(super) tx8: [[u8; 1]; 2],
    pub(super) tx16: [[u8; 2]; 2],
    pub(super) tx32: [[u8; 3]; 2],
    /// Per context.
    pub(super) skip: [u8; 3],
    /// Per transform size.
    pub(super) coef: [CoefProbs; 4],
}

impl Default for Probs {
    fn default() -> Self {
        Self { tx8: TX_PROBS_8X8, tx16: TX_PROBS_16X16, tx32: TX_PROBS_32X32, skip: SKIP_PROBS, coef: COEF_PROBS }
    }
}

/// What coding a bool costs, in 256ths of a bit, at each probability of it
/// being false.
struct Costs([u32; 256]);

impl Costs {
    fn new() -> Self {
        Self(std::array::from_fn(|p| if p == 0 { 0 } else { bool_writer::cost(false, p as u8) }))
    }

    /// Coding `ct[0]` false and `ct[1]` true bools at `p`.
    fn branch(&self, ct: [u32; 2], p: u8) -> i64 {
        i64::from(ct[0]) * i64::from(self.0[usize::from(p)]) + i64::from(ct[1]) * i64::from(self.0[256 - usize::from(p)])
    }

    /// What setting an update flag costs over leaving it clear.
    fn flag(&self) -> i64 {
        i64::from(bool_writer::cost(true, DIFF_UPDATE_PROB)) - i64::from(bool_writer::cost(false, DIFF_UPDATE_PROB))
    }
}

/// The probability of false that `ct` measures (libvpx's `get_binary_prob`).
fn binary_prob(ct: [u32; 2]) -> u8 {
    let den = u64::from(ct[0]) + u64::from(ct[1]);
    if den == 0 {
        return 128;
    }
    ((u64::from(ct[0]) * 256 + den / 2) / den).clamp(1, 255) as u8
}

/// Where `new` is coded as an update of `old` (libvpx's `remap_prob`): the
/// distance between them, folded to the nearer end of the range.
fn update_index(new: u8, old: u8) -> u32 {
    let (v, m) = (i32::from(new) - 1, i32::from(old) - 1);
    let recenter = |v: i32, m: i32| if v > 2 * m { v } else if v >= m { (v - m) * 2 } else { (m - v) * 2 - 1 };
    let i = if 2 * m <= 255 { recenter(v, m) } else { recenter(254 - v, 254 - m) } - 1;
    u32::from(UPDATE_INDEX[i as usize])
}

/// How many bits `index` is coded in by `write_update`.
fn update_bits(index: u32) -> i64 {
    match index {
        0..16 => 5,
        16..32 => 6,
        32..64 => 8,
        64..129 => 10,
        _ => 11,
    }
}

/// Code `new` as an update of `old` (libvpx's `encode_term_subexp`): a prefix
/// of ones saying how large the index is, then the index within it, each
/// handed to `put` as a value and how many bits it is written in.
fn code_update(new: u8, old: u8, mut put: impl FnMut(u32, u32)) {
    let index = update_index(new, old);
    match index {
        0..16 => {
            put(0, 1);
            put(index, 4);
        }
        16..32 => {
            put(0b10, 2);
            put(index - 16, 4);
        }
        32..64 => {
            put(0b110, 3);
            put(index - 32, 5);
        }
        _ => {
            put(0b111, 3);
            // The rest in seven bits, or in eight past the first 65.
            let v = index - 64;
            if v < 65 {
                put(v, 7);
            } else {
                put(65 + ((v - 65) >> 1), 7);
                put((v - 65) & 1, 1);
            }
        }
    }
}

/// `code_update` into `w`.
fn write_update(w: &mut BoolWriter, new: u8, old: u8) {
    code_update(new, old, |value, bits| w.literal(value, bits));
}

/// The best update of `old`, a probability coding `ct`, and what it saves
/// over keeping `old`, its flag and its coding paid for; `old` and 0 when
/// none saves anything (libvpx's `vp9_prob_diff_update_savings_search`).
fn search(costs: &Costs, ct: [u32; 2], old: u8) -> (u8, i64) {
    let before = costs.branch(ct, old);
    let mut best = (old, 0);
    if before > costs.flag() + LEAST_UPDATE {
        let start = binary_prob(ct);
        let step: i16 = if start > old { -1 } else { 1 };
        let mut new = start;
        while new != old {
            let savings = before - costs.branch(ct, new) - (update_bits(update_index(new, old)) << 8) - costs.flag();
            if savings > best.1 {
                best = (new, savings);
            }
            new = (i16::from(new) + step) as u8;
        }
    }
    best
}

/// `search` for the third node, whose probability also sets the eight after
/// it, so that what it saves is counted over all nine (libvpx's
/// `vp9_prob_diff_update_savings_search_model`).
fn search_pivot(costs: &Costs, ct: &[[u32; 2]; 11], old: u8) -> (u8, i64) {
    let cost = |p: u8| costs.branch(ct[PIVOT_NODE], p) + ct[PIVOT_NODE + 1..].iter().zip(PARETO8_FULL[usize::from(p) - 1]).map(|(&c, q)| costs.branch(c, q)).sum::<i64>();
    let before = cost(old);
    let mut best = (old, 0);
    if before > costs.flag() + LEAST_UPDATE {
        let start = binary_prob(ct[PIVOT_NODE]);
        let step: i16 = if start > old { -1 } else { 1 };
        let mut new = start;
        while new != old {
            let savings = before - cost(new) - (update_bits(update_index(new, old)) << 8) - costs.flag();
            if savings > best.1 {
                best = (new, savings);
            }
            new = (i16::from(new) + step) as u8;
        }
    }
    best
}

/// Update `p`, a probability coding `ct`, where that saves more than it
/// costs, and write its flag and the update (libvpx's
/// `vp9_cond_prob_diff_update`).
fn update_one(w: &mut BoolWriter, costs: &Costs, ct: [u32; 2], p: &mut u8) {
    let (new, saved) = search(costs, ct, *p);
    let update = saved > 0 && new != *p;
    w.write(update, DIFF_UPDATE_PROB);
    if update {
        write_update(w, new, *p);
        *p = new;
    }
}

impl Probs {
    /// The frame's table of probabilities: each fixed one as itself, then
    /// per kind of transform block, band and context the token tree's eleven
    /// nodes, the last eight from the third (libvpx's `vp9_pareto8_full`).
    pub(super) fn table(&self) -> Vec<u8> {
        let mut table: Vec<u8> = (0..FIXED).map(|p| p as u8).collect();
        table.extend(self.tx8.iter().flatten().chain(self.tx16.iter().flatten()).chain(self.tx32.iter().flatten()).chain(&self.skip));
        for kind in self.coef.iter().flatten().flatten() {
            for probs in kind.iter().flatten() {
                table.extend(probs);
                table.extend(PARETO8_FULL[usize::from(probs[PIVOT_NODE]).max(1) - 1]);
            }
        }
        table
    }

    /// Write the transform size probabilities' part of a compressed header,
    /// and make its updates (libvpx's `tx_mode_probs`).
    pub(super) fn update_tx(&mut self, w: &mut BoolWriter, counts: &Counts) {
        let costs = Costs::new();
        let probs = self.tx8.iter_mut().flatten().chain(self.tx16.iter_mut().flatten()).chain(self.tx32.iter_mut().flatten());
        for (p, &ct) in probs.zip(&counts.bools[..SKIP_SLOTS]) {
            update_one(w, &costs, ct, p);
        }
    }

    /// Write the skip probabilities' part of a compressed header, and make
    /// its updates.
    pub(super) fn update_skip(&mut self, w: &mut BoolWriter, counts: &Counts) {
        let costs = Costs::new();
        for (p, &ct) in self.skip.iter_mut().zip(&counts.bools[SKIP_SLOTS..]) {
            update_one(w, &costs, ct, p);
        }
    }

    /// Write the coefficient probabilities' part of a compressed header for
    /// transform sizes up to `max_tx`, and make its updates: per size,
    /// whether any probability is updated and then, if so, per probability,
    /// whether it is and to what. A probability is updated where the tokens
    /// of `counts` would have cost less at the update than at it by more
    /// than the update costs, and a size's probabilities where all their
    /// updates together save more than their flags cost.
    pub(super) fn update_coefs(&mut self, w: &mut BoolWriter, counts: &Counts, max_tx: usize) {
        let costs = Costs::new();
        let zero_flag = i64::from(bool_writer::cost(false, DIFF_UPDATE_PROB));
        for tx in 0..=max_tx {
            // Each probability's update, in the order they are written, and
            // what all of them save over sending none.
            let mut updates = Vec::new();
            let mut savings = 0;
            for plane in 0..2 {
                for reference in 0..2 {
                    for band in 0..6 {
                        for ctx in 0..if band == 0 { 3 } else { 6 } {
                            let ct = counts.branches(tx, plane, reference, band, ctx);
                            let old = self.coef[tx][plane][reference][band][ctx];
                            for node in 0..3 {
                                let (new, saved) = if node == PIVOT_NODE { search_pivot(&costs, &ct, old[node]) } else { search(&costs, ct[node], old[node]) };
                                let update = (saved > 0 && new != old[node]).then_some(new);
                                savings += update.map_or(0, |_| saved) - zero_flag;
                                updates.push(update);
                            }
                        }
                    }
                }
            }
            if updates.iter().all(Option::is_none) || savings < 0 {
                w.bit(false);
                continue;
            }
            w.bit(true);
            let mut updates = updates.into_iter();
            for plane in &mut self.coef[tx] {
                for kind in plane {
                    for (band, contexts) in kind.iter_mut().enumerate() {
                        for probs in &mut contexts[..if band == 0 { 3 } else { 6 }] {
                            for p in probs {
                                let update = updates.next().expect("an update per probability");
                                w.write(update.is_some(), DIFF_UPDATE_PROB);
                                if let Some(new) = update {
                                    write_update(w, new, *p);
                                    *p = new;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::super::tables::UPDATE_INDEX_INVERSE;
    use super::*;

    /// An update as a decoder reads one (libvpx's `vp9_diff_update_prob`,
    /// from its flag on): what `old` becomes.
    pub(in crate::encoder) fn read_update(r: &mut BoolReader, old: u8) -> u8 {
        if !r.read(DIFF_UPDATE_PROB) {
            return old;
        }
        let index = if !r.read(128) {
            r.literal(4)
        } else if !r.read(128) {
            r.literal(4) + 16
        } else if !r.read(128) {
            r.literal(5) + 32
        } else {
            let v = r.literal(7);
            (if v < 65 { v } else { (v << 1) - 65 + r.literal(1) }) + 64
        };
        let v = i32::from(UPDATE_INDEX_INVERSE[index as usize]);
        let m = i32::from(old) - 1;
        let inv_recenter = |v: i32, m: i32| if v > 2 * m { v } else if v & 1 == 1 { m - ((v + 1) >> 1) } else { m + (v >> 1) };
        (if 2 * m <= 255 { 1 + inv_recenter(v, m) } else { 255 - inv_recenter(v, 254 - m) }) as u8
    }

    #[test]
    fn every_update_reads_back() {
        for old in 1..=255u8 {
            let mut w = BoolWriter::new();
            for new in (1..=255u8).filter(|&new| new != old) {
                w.write(true, DIFF_UPDATE_PROB);
                write_update(&mut w, new, old);
            }
            let data = w.finish();
            let mut r = BoolReader::new(&data);
            for new in (1..=255u8).filter(|&new| new != old) {
                assert_eq!(read_update(&mut r, old), new, "{old} to {new}");
            }
        }
    }

    #[test]
    fn an_update_costs_the_bits_it_is_written_in() {
        for old in 1..=255u8 {
            for new in (1..=255u8).filter(|&new| new != old) {
                let mut bits = 0;
                code_update(new, old, |_, n| bits += n);
                assert_eq!(i64::from(bits), update_bits(update_index(new, old)), "{old} to {new}");
            }
        }
    }

    #[test]
    fn a_skewed_count_is_worth_an_update_and_an_even_one_is_not() {
        let costs = Costs::new();
        let (new, saved) = search(&costs, [9000, 1000], 128);
        assert!(saved > 0 && (220..=235).contains(&new), "{new} saving {saved}");
        assert_eq!(search(&costs, [5000, 5000], 128), (128, 0));
        assert_eq!(search(&costs, [3, 1], 128), (128, 0));
    }
}
