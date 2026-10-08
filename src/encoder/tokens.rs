//! A transform block's quantized levels as VP9 tokens (libvpx's
//! `decode_coefs`, from the writing side): in scan order, each a token from
//! a tree whose first three probabilities depend on the band of the position
//! and the energy of the neighbours already coded, the rest on the third.

use super::bool_writer::BoolWriter;
use super::tables::{
    CAT1_PROB, CAT2_PROB, CAT3_PROB, CAT4_PROB, CAT5_PROB, CAT6_PROB, COEF_PROBS, COEFBAND_4X4, COEFBAND_8X8PLUS, PARETO8_FULL, SCAN_4X4, SCAN_4X4_NEIGHBORS, SCAN_8X8,
    SCAN_8X8_NEIGHBORS, SCAN_16X16, SCAN_16X16_NEIGHBORS, SCAN_32X32, SCAN_32X32_NEIGHBORS,
};

/// The DCT scan of transform size `tx` (0 for 4×4 to 3 for 32×32): the
/// coefficient at each position, and the two earlier ones each position's
/// context is taken from.
pub(super) fn scan(tx: usize) -> (&'static [u16], &'static [u16]) {
    match tx {
        0 => (&SCAN_4X4, &SCAN_4X4_NEIGHBORS),
        1 => (&SCAN_8X8, &SCAN_8X8_NEIGHBORS),
        2 => (&SCAN_16X16, &SCAN_16X16_NEIGHBORS),
        _ => (&SCAN_32X32, &SCAN_32X32_NEIGHBORS),
    }
}

/// Code `n` bits of `value`, most significant first, each at its own probability.
fn extra(w: &mut BoolWriter, value: i32, probs: &[u8]) {
    for (i, &p) in probs.iter().enumerate() {
        w.write(value >> (probs.len() - 1 - i) & 1 == 1, p);
    }
}

/// Code a level of 2 or more: the token, from the tree under the third
/// probability, then the token's extra bits. Returns the token's energy.
fn large(w: &mut BoolWriter, level: i32, p: &[u8; 8]) -> u8 {
    if level <= 4 {
        w.write(false, p[0]);
        if level == 2 {
            w.write(false, p[1]);
            return 2;
        }
        w.write(true, p[1]);
        w.write(level == 4, p[2]);
        return 3;
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
        return 4;
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
    5
}

/// Code the `levels` (raster order) of a transform block of size `tx` whose
/// scan ends at `eob`, in a block that is intra or `inter`, of luma or
/// chroma, starting from context `ctx`: how many of the blocks above and to
/// the left had coefficients.
pub(super) fn write(w: &mut BoolWriter, tx: usize, chroma: bool, inter: bool, ctx: usize, levels: &[i32], eob: usize) {
    let probs = &COEF_PROBS[tx][usize::from(chroma)][usize::from(inter)];
    let (scan, neighbors) = scan(tx);
    let bands: &[u8] = if tx == 0 { &COEFBAND_4X4 } else { &COEFBAND_8X8PLUS };
    let max = 16 << (2 * tx);
    // The energy of each coefficient coded so far, by raster position.
    let mut energy = [0u8; 1024];
    let mut ctx = ctx;
    // An end of block cannot follow a zero, so after one it is not coded.
    let mut after_zero = false;
    for c in 0..eob {
        let p = &probs[usize::from(bands[c])][ctx];
        if !after_zero {
            w.write(true, p[0]);
        }
        let rc = usize::from(scan[c]);
        let level = levels[rc];
        if level == 0 {
            w.write(false, p[1]);
            energy[rc] = 0;
            after_zero = true;
        } else {
            w.write(true, p[1]);
            let abs = level.abs();
            energy[rc] = if abs == 1 {
                w.write(false, p[2]);
                1
            } else {
                w.write(true, p[2]);
                large(w, abs, &PARETO8_FULL[usize::from(p[2]) - 1])
            };
            w.bit(level < 0);
            after_zero = false;
        }
        let next = c + 1;
        if next < max {
            let (a, b) = (usize::from(neighbors[2 * next]), usize::from(neighbors[2 * next + 1]));
            ctx = (1 + usize::from(energy[a]) + usize::from(energy[b])) >> 1;
        }
    }
    if eob < max {
        w.write(false, probs[usize::from(bands[eob])][ctx][0]);
    }
}

#[cfg(test)]
mod tests {
    use super::super::bool_writer::tests::BoolReader;
    use super::*;

    /// libvpx's `decode_coefs`, read with the specification's bool reader:
    /// the levels back in raster order and the end of block.
    fn read(r: &mut BoolReader, tx: usize, ctx: usize) -> (Vec<i32>, usize) {
        let probs = &COEF_PROBS[tx][0][0];
        let (scan, neighbors) = scan(tx);
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
    fn levels_of_every_size_and_token_read_back() {
        // Every token's range, zeros between, a full block and an empty one.
        let samples = [1, -1, 2, 3, -4, 5, 6, -7, 10, 11, 18, -19, 34, 35, 66, -67, 300, 16_449];
        for tx in 0..4 {
            let n = 4 << tx;
            let mut cases: Vec<Vec<i32>> = vec![vec![0; n * n]];
            let mut sparse = vec![0; n * n];
            for (i, &s) in samples.iter().enumerate() {
                sparse[(i * 7) % (n * n)] = s;
            }
            cases.push(sparse);
            cases.push((0..n * n).map(|i| samples[i % samples.len()]).collect());
            for (case, levels) in cases.iter().enumerate() {
                let (scan, _) = scan(tx);
                let eob = scan.iter().rposition(|&rc| levels[usize::from(rc)] != 0).map_or(0, |i| i + 1);
                for ctx in 0..3 {
                    let mut w = BoolWriter::new();
                    write(&mut w, tx, false, false, ctx, levels, eob);
                    w.literal(0x5a, 7);
                    let data = w.finish();
                    let mut r = BoolReader::new(&data);
                    let (back, back_eob) = read(&mut r, tx, ctx);
                    assert_eq!((&back, back_eob), (levels, eob), "{n}x{n}, case {case}, context {ctx}");
                    assert_eq!(r.literal(7), 0x5a, "{n}x{n}, case {case}: what follows the block");
                }
            }
        }
    }
}
