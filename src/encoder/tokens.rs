//! A transform block's quantized levels as VP9 tokens (libvpx's
//! `decode_coefs`, from the writing side): in scan order, each a token from
//! a tree whose first three probabilities depend on the band of the position
//! and the energy of the neighbours already coded, the rest on the third.
//! The tokens are recorded with their bands and contexts, coded from the
//! record once the frame's probabilities are chosen, and costed beforehand
//! to choose a block's transform size.

use super::bool_writer::{Run, cost};
use super::probs::{CONTEXTS, NODES, Probs, token_context};
use super::tables::{
    CAT1_PROB, CAT2_PROB, CAT3_PROB, CAT4_PROB, CAT5_PROB, CAT6_PROB, COEFBAND_4X4, COEFBAND_8X8PLUS, COL_ISCAN_4X4, COL_ISCAN_8X8, COL_ISCAN_16X16, COL_SCAN_4X4,
    COL_SCAN_4X4_NEIGHBORS, COL_SCAN_8X8, COL_SCAN_8X8_NEIGHBORS, COL_SCAN_16X16, COL_SCAN_16X16_NEIGHBORS, ISCAN_4X4, ISCAN_8X8, ISCAN_16X16, ISCAN_32X32, PARETO8_FULL,
    PT_ENERGY_CLASS, ROW_ISCAN_4X4, ROW_ISCAN_8X8, ROW_ISCAN_16X16, ROW_SCAN_4X4, ROW_SCAN_4X4_NEIGHBORS, ROW_SCAN_8X8, ROW_SCAN_8X8_NEIGHBORS, ROW_SCAN_16X16,
    ROW_SCAN_16X16_NEIGHBORS, SCAN_4X4, SCAN_4X4_NEIGHBORS, SCAN_8X8, SCAN_8X8_NEIGHBORS, SCAN_16X16, SCAN_16X16_NEIGHBORS, SCAN_32X32, SCAN_32X32_NEIGHBORS,
};
use super::tape::Tape;
use super::transform::TxType;

/// The tokens: `ZERO_TOKEN` to `CATEGORY6_TOKEN` in libvpx's order, then the
/// end of block.
pub(super) const TOKENS: usize = 12;
/// The end of block's place among `TOKENS`.
pub(super) const EOB_TOKEN: usize = 11;

/// The token of each level from 0 to 67, the least `CATEGORY6_TOKEN` codes.
const TOKEN_OF: [u8; 68] = {
    let mut table = [10; 68];
    let mut level = 0;
    while level < 67 {
        table[level] = match level {
            0..=4 => level as u8,
            5..=6 => 5,
            7..=10 => 6,
            11..=18 => 7,
            19..=34 => 8,
            _ => 9,
        };
        level += 1;
    }
    table
};

/// The order a transform block's coefficients are coded in: the
/// coefficient at each position, the two earlier ones each position's
/// context is taken from, and per coefficient one past its position.
#[derive(Clone, Copy)]
pub(super) struct Scan {
    pub(super) order: &'static [u16],
    pub(super) neighbors: &'static [u16],
    pub(super) inverse: &'static [u16],
}

/// The scan of transform size `tx` (0 for 4×4 to 3 for 32×32) and type
/// `tx_type` (libvpx's `vp9_scan_orders`).
pub(super) fn scan(tx: usize, tx_type: TxType) -> Scan {
    let at = |order, neighbors, inverse| Scan { order, neighbors, inverse };
    match (tx, tx_type) {
        (0, TxType::AdstDct) => at(&ROW_SCAN_4X4, &ROW_SCAN_4X4_NEIGHBORS, &ROW_ISCAN_4X4),
        (0, TxType::DctAdst) => at(&COL_SCAN_4X4, &COL_SCAN_4X4_NEIGHBORS, &COL_ISCAN_4X4),
        (0, _) => at(&SCAN_4X4, &SCAN_4X4_NEIGHBORS, &ISCAN_4X4),
        (1, TxType::AdstDct) => at(&ROW_SCAN_8X8, &ROW_SCAN_8X8_NEIGHBORS, &ROW_ISCAN_8X8),
        (1, TxType::DctAdst) => at(&COL_SCAN_8X8, &COL_SCAN_8X8_NEIGHBORS, &COL_ISCAN_8X8),
        (1, _) => at(&SCAN_8X8, &SCAN_8X8_NEIGHBORS, &ISCAN_8X8),
        (2, TxType::AdstDct) => at(&ROW_SCAN_16X16, &ROW_SCAN_16X16_NEIGHBORS, &ROW_ISCAN_16X16),
        (2, TxType::DctAdst) => at(&COL_SCAN_16X16, &COL_SCAN_16X16_NEIGHBORS, &COL_ISCAN_16X16),
        (2, _) => at(&SCAN_16X16, &SCAN_16X16_NEIGHBORS, &ISCAN_16X16),
        _ => at(&SCAN_32X32, &SCAN_32X32_NEIGHBORS, &ISCAN_32X32),
    }
}

/// Code `n` bits of `value`, most significant first, each at its own probability.
#[inline(always)]
fn extra(w: &mut Run, value: i32, probs: &[u8]) {
    for (i, &p) in probs.iter().enumerate() {
        w.write(value >> (probs.len() - 1 - i) & 1 == 1, p);
    }
}

/// Code a level of 2 or more: the token, from the tree under the third
/// node at the probabilities `p` of the eight after it, then the token's
/// extra bits.
#[inline(always)]
fn large(w: &mut Run, level: i32, p: &[u8]) {
    if level <= 4 {
        w.write(false, p[0]);
        if level == 2 {
            w.write(false, p[1]);
            return;
        }
        w.write(true, p[1]);
        w.write(level == 4, p[2]);
        return;
    }
    w.write(true, p[0]);
    if level <= 10 {
        w.write(false, p[3]);
        if level <= 6 {
            w.write(false, p[4]);
            extra(w, level - 5, &CAT1_PROB);
        } else {
            w.write(true, p[4]);
            extra(w, level - 7, &CAT2_PROB);
        }
        return;
    }
    w.write(true, p[3]);
    if level <= 34 {
        w.write(false, p[5]);
        if level <= 18 {
            w.write(false, p[6]);
            extra(w, level - 11, &CAT3_PROB);
        } else {
            w.write(true, p[6]);
            extra(w, level - 19, &CAT4_PROB);
        }
    } else {
        w.write(true, p[5]);
        if level <= 66 {
            w.write(false, p[7]);
            extra(w, level - 35, &CAT5_PROB);
        } else {
            w.write(true, p[7]);
            extra(w, level - 67, &CAT6_PROB);
        }
    }
}

/// Code the token of `level` at the probabilities `p` of the token tree's
/// nodes, after the end of block's node when `more`, and its sign.
#[inline(always)]
pub(super) fn code(w: &mut Run, more: bool, level: i32, p: &[u8; NODES]) {
    if more {
        w.write(true, p[0]);
    }
    if level == 0 {
        w.write(false, p[1]);
        return;
    }
    w.write(true, p[1]);
    let abs = level.abs();
    if abs == 1 {
        w.write(false, p[2]);
    } else {
        w.write(true, p[2]);
        large(w, abs, &p[3..]);
    }
    w.write(level < 0, 128);
}

/// What each token costs at a frame's probabilities, in 256ths of a bit, per
/// band and context (`probs::token_context`): each token's way down the
/// tree below the end of block's node, its sign and its extra bits counted
/// as a bit each; then the end of block; then the end of block's node passed
/// on the way to a token.
pub(super) struct Costs(Box<[[u16; TOKENS + 1]; CONTEXTS]>);

impl Costs {
    pub(super) fn new(probs: &Probs) -> Self {
        let mut costs = Box::new([[0; TOKENS + 1]; CONTEXTS]);
        let false_at: [u32; 256] = std::array::from_fn(|p| if p == 0 { 0 } else { cost(false, p as u8) });
        let contexts = probs.coef.iter().flatten().flatten().flat_map(|kind| kind.iter().flatten());
        for (costs, p) in costs.iter_mut().zip(contexts) {
            let q = PARETO8_FULL[usize::from(p[2]).max(1) - 1];
            // Band 0's last three contexts are never coded, and zero.
            let one = |b: bool, p: u8| {
                let p = usize::from(p.max(1));
                if b { false_at[256 - p] } else { false_at[p] }
            };
            let c = |bits: &[(bool, u8)], extra: u32| bits.iter().map(|&(b, p)| one(b, p)).sum::<u32>() + 256 * extra;
            let large = c(&[(true, p[1]), (true, p[2])], 1);
            *costs = [
                c(&[(false, p[1])], 0),
                c(&[(true, p[1]), (false, p[2])], 1),
                large + c(&[(false, q[0]), (false, q[1])], 0),
                large + c(&[(false, q[0]), (true, q[1]), (false, q[2])], 0),
                large + c(&[(false, q[0]), (true, q[1]), (true, q[2])], 0),
                large + c(&[(true, q[0]), (false, q[3]), (false, q[4])], 1),
                large + c(&[(true, q[0]), (false, q[3]), (true, q[4])], 2),
                large + c(&[(true, q[0]), (true, q[3]), (false, q[5]), (false, q[6])], 3),
                large + c(&[(true, q[0]), (true, q[3]), (false, q[5]), (true, q[6])], 4),
                large + c(&[(true, q[0]), (true, q[3]), (true, q[5]), (false, q[7])], 5),
                large + c(&[(true, q[0]), (true, q[3]), (true, q[5]), (true, q[7])], 14),
                c(&[(false, p[0])], 0),
                c(&[(true, p[0])], 0),
            ]
            .map(|v| v.min(u32::from(u16::MAX)) as u16);
        }
        Self(costs)
    }
}

/// What `write` would record costs at `costs`, in 256ths of a bit.
pub(super) fn estimate(costs: &Costs, kind: Kind, ctx: usize, levels: &[i32], eob: usize, energy: &mut [u8; 1024]) -> u32 {
    let tx = kind.tx;
    let first = token_context(tx, kind.chroma, kind.inter);
    let Scan { order: scan, neighbors, .. } = scan(tx, kind.tx_type);
    let bands: &[u8] = if tx == 0 { &COEFBAND_4X4 } else { &COEFBAND_8X8PLUS };
    let max = 16 << (2 * tx);
    let mut ctx = ctx;
    let mut more = true;
    let mut total = 0;
    for c in 0..eob {
        let rc = usize::from(scan[c]);
        let level = levels[rc];
        let token = usize::from(TOKEN_OF[(level.unsigned_abs() as usize).min(67)]);
        energy[rc] = PT_ENERGY_CLASS[token];
        let costs = &costs.0[first + usize::from(bands[c]) * 6 + ctx];
        total += u32::from(costs[token]) + if more { u32::from(costs[TOKENS]) } else { 0 };
        more = level != 0;
        let next = c + 1;
        if next < max {
            let (a, b) = (usize::from(neighbors[2 * next]), usize::from(neighbors[2 * next + 1]));
            ctx = (1 + usize::from(energy[a]) + usize::from(energy[b])) >> 1;
        }
    }
    if eob < max {
        total += u32::from(costs.0[first + usize::from(bands[eob]) * 6 + ctx][EOB_TOKEN]);
    }
    total
}

/// How a transform block's tokens are coded: the probabilities of its
/// size, as log2 less 2, whether it is chroma and whether its block is
/// inter, in the scan of its size and type.
#[derive(Clone, Copy)]
pub(super) struct Kind {
    pub(super) tx: usize,
    pub(super) tx_type: TxType,
    pub(super) chroma: bool,
    pub(super) inter: bool,
}

/// Record the `levels` (raster order) of a transform block of `kind` whose
/// scan ends at `eob`, starting from context `ctx`: how many of the blocks
/// above and to the left had coefficients. Each token is recorded with the
/// band and context it is coded at, which come from the energy of the
/// tokens before it. `energy` is scratch, its contents whatever they are.
pub(super) fn write(t: &mut Tape, kind: Kind, ctx: usize, levels: &[i32], eob: usize, energy: &mut [u8; 1024]) {
    let tx = kind.tx;
    let first = token_context(tx, kind.chroma, kind.inter);
    let Scan { order: scan, neighbors, .. } = scan(tx, kind.tx_type);
    let bands: &[u8] = if tx == 0 { &COEFBAND_4X4 } else { &COEFBAND_8X8PLUS };
    let max = 16 << (2 * tx);
    // The energy of each coefficient coded so far, by raster position: a
    // position's context is read from neighbours earlier in the scan, so
    // only from what this block has written.
    let mut ctx = ctx;
    // An end of block cannot follow a zero, so after one it is not coded.
    let mut more = true;
    for c in 0..eob {
        let rc = usize::from(scan[c]);
        let level = levels[rc];
        let token = usize::from(TOKEN_OF[(level.unsigned_abs() as usize).min(67)]);
        energy[rc] = PT_ENERGY_CLASS[token];
        t.token(first + usize::from(bands[c]) * 6 + ctx, more, level, token);
        more = level != 0;
        let next = c + 1;
        if next < max {
            let (a, b) = (usize::from(neighbors[2 * next]), usize::from(neighbors[2 * next + 1]));
            ctx = (1 + usize::from(energy[a]) + usize::from(energy[b])) >> 1;
        }
    }
    if eob < max {
        t.end(first + usize::from(bands[eob]) * 6 + ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::super::tables::COEF_PROBS;
    use super::*;

    /// libvpx's `decode_coefs`, read with the specification's bool reader:
    /// the levels back in raster order and the end of block.
    fn read(r: &mut BoolReader, tx: usize, tx_type: TxType, ctx: usize) -> (Vec<i32>, usize) {
        let probs = &COEF_PROBS[tx][0][0];
        let Scan { order: scan, neighbors, .. } = scan(tx, tx_type);
        let bands: &[u8] = if tx == 0 { &COEFBAND_4X4 } else { &COEFBAND_8X8PLUS };
        let max = 16 << (2 * tx);
        let mut levels = vec![0; max];
        let mut cache = vec![0u8; max];
        let (mut c, mut ctx) = (0, ctx);
        let bits = |r: &mut BoolReader, probs: &[u8]| probs.iter().fold(0, |v, &p| v << 1 | i32::from(r.read(p)));
        while c < max {
            let mut p = probs[usize::from(bands[c])][ctx];
            if !r.read(p[0]) {
                break;
            }
            loop {
                if r.read(p[1]) {
                    break;
                }
                cache[usize::from(scan[c])] = 0;
                c += 1;
                if c == max {
                    return (levels, c);
                }
                ctx = (1 + usize::from(cache[usize::from(neighbors[2 * c])]) + usize::from(cache[usize::from(neighbors[2 * c + 1])])) >> 1;
                p = probs[usize::from(bands[c])][ctx];
            }
            let (level, energy) = if !r.read(p[2]) {
                (1, 1)
            } else {
                let q = PARETO8_FULL[usize::from(p[2]) - 1];
                if !r.read(q[0]) {
                    if !r.read(q[1]) { (2, 2) } else { (3 + i32::from(r.read(q[2])), 3) }
                } else if !r.read(q[3]) {
                    if !r.read(q[4]) { (5 + bits(r, &CAT1_PROB), 4) } else { (7 + bits(r, &CAT2_PROB), 4) }
                } else if !r.read(q[5]) {
                    if !r.read(q[6]) { (11 + bits(r, &CAT3_PROB), 5) } else { (19 + bits(r, &CAT4_PROB), 5) }
                } else if !r.read(q[7]) {
                    (35 + bits(r, &CAT5_PROB), 5)
                } else {
                    (67 + bits(r, &CAT6_PROB), 5)
                }
            };
            let rc = usize::from(scan[c]);
            levels[rc] = if r.read(128) { -level } else { level };
            cache[rc] = energy;
            c += 1;
            if c < max {
                ctx = (1 + usize::from(cache[usize::from(neighbors[2 * c])]) + usize::from(cache[usize::from(neighbors[2 * c + 1])])) >> 1;
            }
        }
        (levels, c)
    }

    #[test]
    fn the_estimate_is_what_the_tokens_code_in() {
        // Levels up to four, which have no extra bits for the estimate to
        // count at even odds: what it says is what coding them takes.
        let probs = Probs::default();
        let costs = Costs::new(&probs);
        for tx in 0..4 {
            let n = 4 << tx;
            let scan = scan(tx, TxType::DctDct).order;
            let mut state = 0x9e37_79b9_u32;
            let mut tape = Tape::new();
            let mut estimate = 0;
            let mut energy = [0; 1024];
            for block in 0..400 / (tx + 1) {
                let mut levels = vec![0; n * n];
                // Denser towards the start of the scan, as coefficients are.
                for (c, &rc) in scan.iter().enumerate() {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    if (state % 1000) as usize > 600 + 400 * c / (n * n) {
                        levels[usize::from(rc)] = (state >> 20) as i32 % 9 - 4;
                    }
                }
                let eob = scan.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
                let kind = Kind { tx, tx_type: TxType::DctDct, chroma: block % 2 == 1, inter: block % 3 == 0 };
                estimate += super::estimate(&costs, kind, 0, &levels, eob, &mut energy);
                write(&mut tape, kind, 0, &levels, eob, &mut energy);
            }
            let coded = super::super::tape::code([&tape], &probs.table()).len() as f64;
            let estimated = f64::from(estimate) / 2048.0;
            assert!((coded - estimated).abs() < coded / 100.0 + 8.0, "{n}x{n}: {coded} bytes coded, {estimated} estimated");
        }
    }

    #[test]
    fn levels_of_every_size_type_and_token_read_back() {
        // Every token's range, zeros between, a full block and an empty one.
        let samples = [1, -1, 2, 3, -4, 5, 6, -7, 10, 11, 18, -19, 34, 35, 66, -67, 300, 16_449];
        let types = [TxType::DctDct, TxType::AdstDct, TxType::DctAdst, TxType::AdstAdst];
        for (tx, tx_type) in (0..3).flat_map(|tx| types.map(|t| (tx, t))).chain([(3, TxType::DctDct)]) {
            let n = 4 << tx;
            let mut cases: Vec<Vec<i32>> = vec![vec![0; n * n]];
            let mut sparse = vec![0; n * n];
            for (i, &s) in samples.iter().enumerate() {
                sparse[(i * 7) % (n * n)] = s;
            }
            cases.push(sparse);
            cases.push((0..n * n).map(|i| samples[i % samples.len()]).collect());
            for (case, levels) in cases.iter().enumerate() {
                let scan = scan(tx, tx_type).order;
                let eob = scan.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
                for ctx in 0..3 {
                    let mut t = Tape::new();
                    // Scratch left as another block would leave it.
                    let mut energy = [(case * 3 + ctx) as u8; 1024];
                    write(&mut t, Kind { tx, tx_type, chroma: false, inter: false }, ctx, levels, eob, &mut energy);
                    for i in (0..7).rev() {
                        t.write(0x5a >> i & 1 == 1, 128);
                    }
                    let data = super::super::tape::code([&t], &Probs::default().table());
                    let mut r = BoolReader::new(&data);
                    let (back, back_eob) = read(&mut r, tx, tx_type, ctx);
                    assert_eq!((&back, back_eob), (levels, eob), "{n}x{n} {tx_type:?}, case {case}, context {ctx}");
                    assert_eq!(r.literal(7), 0x5a, "{n}x{n}, case {case}: what follows the block");
                }
            }
        }
    }
}
