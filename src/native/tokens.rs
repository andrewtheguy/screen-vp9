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
    COL_SCAN_8X8, COL_SCAN_16X16, ISCAN_4X4, ISCAN_8X8, ISCAN_16X16, ISCAN_32X32, PARETO8_FULL,
    ROW_ISCAN_4X4, ROW_ISCAN_8X8, ROW_ISCAN_16X16, ROW_SCAN_4X4, ROW_SCAN_8X8, ROW_SCAN_16X16,
    SCAN_4X4, SCAN_8X8, SCAN_16X16, SCAN_32X32, };
use super::tape::Tape;
use super::transform::TxType;

/// The tokens: `ZERO_TOKEN` to `CATEGORY6_TOKEN` in libvpx's order, then the
/// end of block.
pub(super) const TOKENS: usize = 12;
/// The end of block's place among `TOKENS`.
pub(super) const EOB_TOKEN: usize = 11;

/// The order a transform block's coefficients are coded in: the
/// coefficient at each position, and per coefficient one past its position.
#[derive(Clone, Copy)]
pub(super) struct Scan {
    pub(super) order: &'static [u16],
    pub(super) inverse: &'static [u16],
}

/// The scan of transform size `tx` (0 for 4×4 to 3 for 32×32) and type
/// `tx_type` (libvpx's `vp9_scan_orders`).
pub(super) fn scan(tx: usize, tx_type: TxType) -> Scan {
    let at = |order, inverse| Scan { order, inverse };
    match (tx, tx_type) {
        (0, TxType::AdstDct) => at(&ROW_SCAN_4X4, &ROW_ISCAN_4X4),
        (0, TxType::DctAdst) => at(&COL_SCAN_4X4, &COL_ISCAN_4X4),
        (0, _) => at(&SCAN_4X4, &ISCAN_4X4),
        (1, TxType::AdstDct) => at(&ROW_SCAN_8X8, &ROW_ISCAN_8X8),
        (1, TxType::DctAdst) => at(&COL_SCAN_8X8, &COL_ISCAN_8X8),
        (1, _) => at(&SCAN_8X8, &ISCAN_8X8),
        (2, TxType::AdstDct) => at(&ROW_SCAN_16X16, &ROW_ISCAN_16X16),
        (2, TxType::DctAdst) => at(&COL_SCAN_16X16, &COL_ISCAN_16X16),
        (2, _) => at(&SCAN_16X16, &ISCAN_16X16),
        _ => at(&SCAN_32X32, &ISCAN_32X32),
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
/// as a bit each; then the end of block; and from `AFTER_MORE` each token
/// again with the end of block's node passed on the way to it, which a
/// token costs unless the one before it was a zero. A token's whole cost
/// is under a quarter of what 16 bits hold.
pub(super) struct Costs(Box<[[u16; 2 * AFTER_MORE]; CONTEXTS]>);

/// Where in a row of `Costs` the tokens' costs after the end of block's node
/// start.
const AFTER_MORE: usize = 16;

impl Costs {
    pub(super) fn new(probs: &Probs) -> Self {
        let mut costs = Box::new([[0; 2 * AFTER_MORE]; CONTEXTS]);
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
            let plain = [
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
            costs[..TOKENS].copy_from_slice(&plain[..TOKENS]);
            for (after, token) in costs[AFTER_MORE..].iter_mut().zip(&plain[..EOB_TOKEN]) {
                *after = token.saturating_add(plain[TOKENS]);
            }
        }
        Self(costs)
    }
}

/// One position of a scan, with what its token is coded by: where its
/// coefficient is, six times its band, and how many sixteens of the block in
/// raster order hold the scan as far as here.
#[derive(Clone, Copy)]
struct Place {
    rc: u16,
    band: u8,
    sixteens: u8,
}

/// Which of the coefficients coded before one its context is taken from
/// (libvpx's `init_scan_neighbors`): the one above and the one to the left,
/// or one of them twice where a scan runs along the rows or down the
/// columns. On the first row it is the one to the left either way, and in
/// the first column the one above.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Neighbours {
    Both,
    Left,
    Above,
}

/// The places of the scan of transform size `tx` and type `tx_type`, the
/// scan's tables read once into one, so that a token costs one read of it,
/// and where its contexts come from.
fn places(tx: usize, tx_type: TxType) -> (&'static [Place], Neighbours) {
    static PLACES: std::sync::LazyLock<[Vec<Place>; 10]> = std::sync::LazyLock::new(|| {
        std::array::from_fn(|i| {
            let (tx, tx_type) = (i / 3, [TxType::DctDct, TxType::AdstDct, TxType::DctAdst][i % 3]);
            let Scan { order, .. } = scan(tx, tx_type);
            let bands: &[u8] = if tx == 0 { &COEFBAND_4X4 } else { &COEFBAND_8X8PLUS };
            let mut reach = 0;
            (0..order.len())
                .map(|c| {
                    reach = reach.max(order[c] / 16 + 1);
                    Place { rc: order[c], band: bands[c] * 6, sixteens: reach as u8 }
                })
                .collect()
        })
    });
    let (kind, neighbours) = match tx_type {
        TxType::AdstDct if tx < 3 => (1, Neighbours::Left),
        TxType::DctAdst if tx < 3 => (2, Neighbours::Above),
        _ => (0, Neighbours::Both),
    };
    (&PLACES[tx.min(3) * 3 + kind], neighbours)
}

/// Per coefficient of the first `sixteens` sixteens of a block of size `tx`
/// in raster order, into `map`: its
/// token in the low five bits, and above them the context a token there is
/// coded at, which is the rounded mean of the energies of its neighbours
/// (`PT_ENERGY_CLASS` of their tokens). The first's context is `ctx`, the
/// block's. Sixteen coefficients at a time, so that the loop along the scan
/// only reads.
#[inline(always)]
fn chart(tx: usize, neighbours: Neighbours, ctx: usize, levels: &[i16], sixteens: usize, map: &mut [u8; 1024]) {
    let levels = &levels[..16 * sixteens];
    #[cfg(target_arch = "aarch64")]
    neon::chart(tx, neighbours, levels, map);
    #[cfg(target_arch = "x86_64")]
    avx2::chart(tx, neighbours, levels, map);
    map[0] = map[0] & (CONTEXT - 1) | (ctx as u8) << CONTEXT.trailing_zeros();
}

/// What a context counts in a byte of `chart`'s map, a token under it.
const CONTEXT: u8 = 32;

/// Which of sixteen coefficients in raster order are in a block's first row,
/// from its first sixteen: at 4×4 and at 8×8, then every one.
const FIRST_ROW: [[u8; 16]; 3] = [[255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], [255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0], [255; 16]];
/// Which are in its first column: at 4×4, at 8×8, and where a row starts.
const FIRST_COLUMN: [[u8; 16]; 3] = [[255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0], [255, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0], [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]];

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::*;

    use super::{CONTEXT, FIRST_COLUMN, FIRST_ROW, Neighbours};

    /// `super::chart`, but for the first coefficient's context.
    #[inline(always)]
    pub(super) fn chart(tx: usize, neighbours: Neighbours, levels: &[i16], map: &mut [u8; 1024]) {
        // The crate is built for AVX2, but its intrinsics are still `unsafe`
        // to call from a function not marked for it; and every load and
        // store is of sixteen values inside `levels`, whole sixteens, or
        // `map`, which is as long as the largest block.
        assert!(levels.len() <= 16 << (2 * tx) && tx < 4);
        unsafe {
            let zero = _mm_setzero_si128();
            let (first_row, first_column) = (_mm_loadu_si128(FIRST_ROW[tx.min(2)].as_ptr().cast()), _mm_loadu_si128(FIRST_COLUMN[tx.min(2)].as_ptr().cast()));
            // The energies of the sixteen before these, and of the sixteen
            // before those.
            let mut before = [zero; 2];
            for (k, sixteen) in levels.as_chunks::<16>().0.iter().enumerate() {
                let magnitude = _mm256_abs_epi16(_mm256_loadu_si256(sixteen.as_ptr().cast()));
                // Held to a byte, in order.
                let l = _mm256_castsi256_si128(_mm256_permute4x64_epi64::<0b1000>(_mm256_packus_epi16(magnitude, magnitude)));
                // -1 where a magnitude is over `k`.
                let over = |k: i8| {
                    let least = _mm_set1_epi8(k + 1);
                    _mm_cmpeq_epi8(_mm_max_epu8(l, least), l)
                };
                let over_10 = over(10);
                // Up to 4 a token is its magnitude, and from there one more
                // at each of `TOKEN_OF`'s steps.
                let mut token = _mm_sub_epi8(_mm_min_epu8(l, _mm_set1_epi8(5)), over_10);
                for step in [6, 18, 34, 66] {
                    token = _mm_sub_epi8(token, over(step));
                }
                let energy = _mm_sub_epi8(_mm_sub_epi8(_mm_min_epu8(l, _mm_set1_epi8(3)), over(4)), over_10);
                // Each one's neighbour to the left and the one above, a row
                // back.
                let left = _mm_alignr_epi8::<15>(energy, before[0]);
                let above = match tx {
                    0 => _mm_slli_si128::<4>(energy),
                    1 => _mm_alignr_epi8::<8>(energy, before[0]),
                    2 => before[0],
                    _ => before[1],
                };
                let in_first_row = if 16 * k < 4 << tx { first_row } else { zero };
                let in_first_column = if tx < 3 || k % 2 == 0 { first_column } else { zero };
                let (above, left) = (_mm_blendv_epi8(above, left, in_first_row), _mm_blendv_epi8(left, above, in_first_column));
                let context = match neighbours {
                    Neighbours::Both => _mm_avg_epu8(above, left),
                    Neighbours::Left => left,
                    Neighbours::Above => above,
                };
                // A context is under 8: shifted as 16 bits, it stays in its
                // byte.
                let context = _mm_slli_epi16::<{ CONTEXT.trailing_zeros() as i32 }>(context);
                _mm_storeu_si128(map[16 * k..][..16].as_mut_ptr().cast(), _mm_or_si128(token, context));
                before = [energy, before[0]];
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use std::arch::aarch64::*;

    use super::{CONTEXT, FIRST_COLUMN, FIRST_ROW, Neighbours};

    /// `super::chart`, but for the first coefficient's context.
    #[inline(always)]
    pub(super) fn chart(tx: usize, neighbours: Neighbours, levels: &[i16], map: &mut [u8; 1024]) {
        // The crate is built for NEON, and every load and store is of
        // sixteen values inside `levels`, whole sixteens, or `map`, which
        // is as long as the largest block.
        assert!(levels.len() <= 16 << (2 * tx) && tx < 4);
        unsafe {
            let zero = vdupq_n_u8(0);
            let (first_row, first_column) = (vld1q_u8(FIRST_ROW[tx.min(2)].as_ptr()), vld1q_u8(FIRST_COLUMN[tx.min(2)].as_ptr()));
            // The energies of the sixteen before these, and of the sixteen
            // before those.
            let mut before = [zero; 2];
            for (k, sixteen) in levels.as_chunks::<16>().0.iter().enumerate() {
                // The magnitudes, held to a byte.
                let l = vcombine_u8(vqmovun_s16(vabsq_s16(vld1q_s16(sixteen.as_ptr()))), vqmovun_s16(vabsq_s16(vld1q_s16(sixteen.as_ptr().add(8)))));
                // -1 where a magnitude is over `k`.
                let over = |k: u8| vcgtq_u8(l, vdupq_n_u8(k));
                let over_10 = over(10);
                // Up to 4 a token is its magnitude, and from there one more
                // at each of `TOKEN_OF`'s steps.
                let mut token = vsubq_u8(vminq_u8(l, vdupq_n_u8(5)), over_10);
                for step in [6, 18, 34, 66] {
                    token = vsubq_u8(token, over(step));
                }
                let energy = vsubq_u8(vsubq_u8(vminq_u8(l, vdupq_n_u8(3)), over(4)), over_10);
                // Each one's neighbour to the left and the one above, a row
                // back.
                let left = vextq_u8::<15>(before[0], energy);
                let above = match tx {
                    0 => vextq_u8::<12>(zero, energy),
                    1 => vextq_u8::<8>(before[0], energy),
                    2 => before[0],
                    _ => before[1],
                };
                let in_first_row = if 16 * k < 4 << tx { first_row } else { zero };
                let in_first_column = if tx < 3 || k % 2 == 0 { first_column } else { zero };
                let (above, left) = (vbslq_u8(in_first_row, left, above), vbslq_u8(in_first_column, above, left));
                let context = match neighbours {
                    Neighbours::Both => vrhaddq_u8(above, left),
                    Neighbours::Left => left,
                    Neighbours::Above => above,
                };
                let context = vshlq_n_u8::<{ CONTEXT.trailing_zeros() as i32 }>(context);
                vst1q_u8(map[16 * k..][..16].as_mut_ptr(), vorrq_u8(token, context));
                before = [energy, before[0]];
            }
        }
    }
}

/// What `write` would record costs at `costs`, in 256ths of a bit. `map` is
/// scratch, its contents whatever they are.
pub(super) fn estimate(costs: &Costs, kind: Kind, ctx: usize, levels: &[i16], eob: usize, map: &mut [u8; 1024]) -> u32 {
    let first = token_context(kind.tx, kind.chroma, kind.inter);
    let (places, neighbours) = places(kind.tx, kind.tx_type);
    if eob == 0 {
        return u32::from(costs.0[first + ctx][EOB_TOKEN]);
    }
    chart(kind.tx, neighbours, ctx, levels, usize::from(places[eob.min(places.len() - 1)].sixteens), map);
    let at = |place: &Place| {
        let m = usize::from(map[usize::from(place.rc) & 1023]);
        (first + usize::from(place.band) + m / usize::from(CONTEXT), m % usize::from(CONTEXT))
    };
    // An end of block cannot follow a zero, so after one its node is not
    // passed.
    let mut more = AFTER_MORE;
    let mut total = 0;
    for place in &places[..eob] {
        let (context, token) = at(place);
        total += u32::from(costs.0[context][(more + token) % (2 * AFTER_MORE)]);
        more = if token != 0 { AFTER_MORE } else { 0 };
    }
    if let Some(place) = places.get(eob) {
        total += u32::from(costs.0[at(place).0][EOB_TOKEN]);
    }
    total
}

/// `estimate` from `ctx`, given `bits`, what it says from context 0: the
/// context the blocks above and left give is coded at by the first token
/// alone, or by the end of block of a block with none, and every scan
/// starts at the first coefficient.
pub(super) fn recontext(costs: &Costs, kind: Kind, ctx: usize, levels: &[i16], eob: usize, bits: u32) -> u32 {
    let first = token_context(kind.tx, kind.chroma, kind.inter);
    let column = if eob == 0 { EOB_TOKEN } else { AFTER_MORE + token(levels[0]) };
    bits - u32::from(costs.0[first][column]) + u32::from(costs.0[first + ctx][column])
}

/// The token of a level: the level itself up to four, then
/// `CATEGORY1_TOKEN` to `CATEGORY6_TOKEN` by its size.
fn token(level: i16) -> usize {
    match level.unsigned_abs() {
        l @ 0..=4 => usize::from(l),
        5..=6 => 5,
        7..=10 => 6,
        11..=18 => 7,
        19..=34 => 8,
        35..=66 => 9,
        _ => 10,
    }
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
/// tokens before it. `map` is scratch, its contents whatever they are.
pub(super) fn write(t: &mut Tape, kind: Kind, ctx: usize, levels: &[i16], eob: usize, map: &mut [u8; 1024]) {
    let first = token_context(kind.tx, kind.chroma, kind.inter);
    let (places, neighbours) = places(kind.tx, kind.tx_type);
    if eob == 0 {
        return t.end(first + ctx);
    }
    // A position's context is read from neighbours earlier in the scan, and
    // what is past the end of block is zero.
    chart(kind.tx, neighbours, ctx, levels, usize::from(places[eob.min(places.len() - 1)].sixteens), map);
    let at = |place: &Place| {
        let m = usize::from(map[usize::from(place.rc) & 1023]);
        (first + usize::from(place.band) + m / usize::from(CONTEXT), m % usize::from(CONTEXT))
    };
    // An end of block cannot follow a zero, so after one it is not coded.
    let mut more = true;
    for place in &places[..eob] {
        let (context, token) = at(place);
        t.token(context, more, i32::from(levels[usize::from(place.rc)]), token);
        more = token != 0;
    }
    if let Some(place) = places.get(eob) {
        t.end(at(place).0);
    }
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::super::tables::COEF_PROBS;
    use super::super::tables::{PT_ENERGY_CLASS, COL_SCAN_16X16_NEIGHBORS, COL_SCAN_4X4_NEIGHBORS, COL_SCAN_8X8_NEIGHBORS, ROW_SCAN_16X16_NEIGHBORS, ROW_SCAN_4X4_NEIGHBORS, ROW_SCAN_8X8_NEIGHBORS, SCAN_16X16_NEIGHBORS, SCAN_32X32_NEIGHBORS, SCAN_4X4_NEIGHBORS, SCAN_8X8_NEIGHBORS};
    use super::*;

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

    /// The two earlier coefficients each position of a scan takes its context
    /// from, a pair a position (libvpx's `vp9_scan_orders`' neighbours).
    fn neighbors(tx: usize, tx_type: TxType) -> &'static [u16] {
        match (tx, tx_type) {
            (0, TxType::AdstDct) => &ROW_SCAN_4X4_NEIGHBORS,
            (0, TxType::DctAdst) => &COL_SCAN_4X4_NEIGHBORS,
            (0, _) => &SCAN_4X4_NEIGHBORS,
            (1, TxType::AdstDct) => &ROW_SCAN_8X8_NEIGHBORS,
            (1, TxType::DctAdst) => &COL_SCAN_8X8_NEIGHBORS,
            (1, _) => &SCAN_8X8_NEIGHBORS,
            (2, TxType::AdstDct) => &ROW_SCAN_16X16_NEIGHBORS,
            (2, TxType::DctAdst) => &COL_SCAN_16X16_NEIGHBORS,
            (2, _) => &SCAN_16X16_NEIGHBORS,
            _ => &SCAN_32X32_NEIGHBORS,
        }
    }

    /// `estimate` a token at a time, its context from the scan's own table
    /// of neighbours: what the map is held to.
    fn estimate_by_one(costs: &Costs, kind: Kind, ctx: usize, levels: &[i16], eob: usize) -> u32 {
        let first = token_context(kind.tx, kind.chroma, kind.inter);
        let (Scan { order, .. }, neighbors) = (scan(kind.tx, kind.tx_type), neighbors(kind.tx, kind.tx_type));
        let bands: &[u8] = if kind.tx == 0 { &COEFBAND_4X4 } else { &COEFBAND_8X8PLUS };
        let mut energy = vec![0; order.len()];
        let (mut ctx, mut more, mut total) = (ctx, true, 0);
        for c in 0..eob {
            let rc = usize::from(order[c]);
            let token = usize::from(TOKEN_OF[usize::from(levels[rc].unsigned_abs()).min(67)]);
            energy[rc] = PT_ENERGY_CLASS[token];
            let costs = &costs.0[first + usize::from(bands[c]) * 6 + ctx];
            total += u32::from(costs[token]) + if more { u32::from(costs[AFTER_MORE]) - u32::from(costs[0]) } else { 0 };
            more = levels[rc] != 0;
            if c + 1 < order.len() {
                ctx = (1 + usize::from(energy[usize::from(neighbors[2 * c + 2])]) + usize::from(energy[usize::from(neighbors[2 * c + 3])])) >> 1;
            }
        }
        if eob < order.len() {
            total += u32::from(costs.0[first + usize::from(bands[eob]) * 6 + ctx][EOB_TOKEN]);
        }
        total
    }

    #[test]
    fn the_map_gives_what_a_token_at_a_time_does() {
        let costs = Costs::new(&Probs::default());
        let mut state = 0x1234_5678_u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let types = [TxType::DctDct, TxType::AdstDct, TxType::DctAdst, TxType::AdstAdst];
        for round in 0..4000 {
            let tx = round % 4;
            let tx_type = if tx == 3 { TxType::DctDct } else { types[round / 4 % 4] };
            let n = 4 << tx;
            // Every token's range, and how full a block is, from nearly
            // empty to every coefficient.
            let (range, one_in) = ([2, 5, 12, 80, 20_000][round / 16 % 5], [1, 2, 5, 40][round / 80 % 4]);
            let levels: Vec<i16> = (0..n * n).map(|_| if next() % one_in == 0 { ((next() % (2 * range + 1)) as i32 - range as i32) as i16 } else { 0 }).collect();
            let order = scan(tx, tx_type).order;
            let eob = order.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
            let kind = Kind { tx, tx_type, chroma: round % 3 == 0, inter: round % 5 == 0 };
            let ctx = round % 3;
            let mut map = [(round % 251) as u8; 1024];
            assert_eq!(estimate(&costs, kind, ctx, &levels, eob, &mut map), estimate_by_one(&costs, kind, ctx, &levels, eob), "round {round}: {n}x{n} {tx_type:?}, eob {eob}");
        }
    }

    #[test]
    fn the_estimate_moved_to_a_context_is_the_estimate_from_it() {
        let costs = Costs::new(&Probs::default());
        let mut state = 0x9e37_79b9_u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let types = [TxType::DctDct, TxType::AdstDct, TxType::DctAdst, TxType::AdstAdst];
        for round in 0..4000 {
            let tx = round % 4;
            let tx_type = if tx == 3 { TxType::DctDct } else { types[round / 4 % 4] };
            let n = 4 << tx;
            // Every token at the first coefficient, blocks with nothing
            // in them among the rest.
            let (range, one_in) = ([2, 5, 12, 80, 20_000][round / 16 % 5], [1, 2, 5, 400][round / 80 % 4]);
            let levels: Vec<i16> = (0..n * n).map(|_| if next() % one_in == 0 { ((next() % (2 * range + 1)) as i32 - range as i32) as i16 } else { 0 }).collect();
            let order = scan(tx, tx_type).order;
            let eob = order.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
            let kind = Kind { tx, tx_type, chroma: round % 3 == 0, inter: round % 5 == 0 };
            let mut map = [0u8; 1024];
            let from_0 = estimate(&costs, kind, 0, &levels, eob, &mut map);
            for ctx in 0..3 {
                assert_eq!(recontext(&costs, kind, ctx, &levels, eob, from_0), estimate(&costs, kind, ctx, &levels, eob, &mut map), "round {round}: {n}x{n} {tx_type:?}, eob {eob}, context {ctx}");
            }
        }
    }

    /// libvpx's `decode_coefs`, read with the specification's bool reader:
    /// the levels back in raster order and the end of block.
    fn read(r: &mut BoolReader, tx: usize, tx_type: TxType, ctx: usize) -> (Vec<i32>, usize) {
        let probs = &COEF_PROBS[tx][0][0];
        let (Scan { order: scan, .. }, neighbors) = (scan(tx, tx_type), neighbors(tx, tx_type));
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
                let mut levels = vec![0i16; n * n];
                // Denser towards the start of the scan, as coefficients are.
                for (c, &rc) in scan.iter().enumerate() {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    if (state % 1000) as usize > 600 + 400 * c / (n * n) {
                        levels[usize::from(rc)] = ((state >> 20) as i32 % 9 - 4) as i16;
                    }
                }
                let eob = scan.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
                let kind = Kind { tx, tx_type: TxType::DctDct, chroma: block % 2 == 1, inter: block % 3 == 0 };
                estimate += super::estimate(&costs, kind, 0, &levels, eob, &mut energy);
                write(&mut tape, kind, 0, &levels, eob, &mut energy);
            }
            let coded = super::super::tape::code([&tape], &probs.table(), None).len() as f64;
            let estimated = f64::from(estimate) / 2048.0;
            assert!((coded - estimated).abs() < coded / 100.0 + 8.0, "{n}x{n}: {coded} bytes coded, {estimated} estimated");
        }
    }

    #[test]
    fn levels_of_every_size_type_and_token_read_back() {
        // Every token's range, zeros between, a full block and an empty one.
        let samples: [i16; 18] = [1, -1, 2, 3, -4, 5, 6, -7, 10, 11, 18, -19, 34, 35, 66, -67, 300, 16_449];
        let types = [TxType::DctDct, TxType::AdstDct, TxType::DctAdst, TxType::AdstAdst];
        for (tx, tx_type) in (0..3).flat_map(|tx| types.map(|t| (tx, t))).chain([(3, TxType::DctDct)]) {
            let n = 4 << tx;
            let mut cases: Vec<Vec<i16>> = vec![vec![0; n * n]];
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
                    let data = super::super::tape::code([&t], &Probs::default().table(), None);
                    let mut r = BoolReader::new(&data);
                    let (back, back_eob) = read(&mut r, tx, tx_type, ctx);
                    let back: Vec<i16> = back.iter().map(|&v| v as i16).collect();
                    assert_eq!((&back, back_eob), (levels, eob), "{n}x{n} {tx_type:?}, case {case}, context {ctx}");
                    assert_eq!(r.literal(7), 0x5a, "{n}x{n}, case {case}: what follows the block");
                }
            }
        }
    }
}
