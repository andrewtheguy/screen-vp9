//! Whole-pixel motion search for what moves on a screen: scrolls, dragged
//! windows, anything copied whole from one place to another.
//!
//! A frame's luma is hashed over 8×8 windows at every pixel, and a window
//! whose hash, scrambled, falls in one sixty-fourth of its range is an anchor:
//! which windows are anchors depends on their content alone, so content that
//! moves takes its anchors with it. An anchor of the frame being coded whose
//! hash is an anchor's of the frame before gives the vector between the two,
//! and each superblock offers the inter blocks in it the vectors its own
//! anchors give most, and those the whole frame's give most. A vector found
//! here is only a guess until a block is measured at it.
//!
//! A window that is the same one pixel across or down, a flat one or a
//! stripe, matches wherever there is more of it and is never an anchor; nor
//! is a hash the frame before has more than `MAX_REPEATS` anchors of.
//!
//! Hashes are kept per 8×8 of the frame and taken again only where the
//! source changed, so a frame that changes little costs little; they are
//! taken and looked up in a band of 8×8 rows on each thread.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use super::mv::{MAX_PIXELS, Mv};
use super::{Geometry, on_threads};

/// The vectors a superblock offers: its own anchors' best two, then the
/// frame's best two.
pub(super) const VECTORS: usize = 4;

/// A window is an anchor when the top `ANCHOR_BITS` bits of its hash times
/// `SCRAMBLE` are zero.
const ANCHOR_BITS: u32 = 6;
const SCRAMBLE: u32 = 0x2545_f491;

/// The anchors kept per 8×8; any more starting in one 8×8 are dropped.
const PER_8X8: usize = 2;

/// A hash the frame before has more anchors of than this repeats too much to
/// say where anything came from.
const MAX_REPEATS: usize = 4;

/// How many anchors must agree on a vector for a superblock to offer it.
const OWN_VOTES: usize = 2;

/// How many anchors must agree on a vector for the whole frame to offer it,
/// and the share of the anchors looked up that must, `1 / FRAME_SHARE`: what
/// every superblock is offered has to be something that moved, not what
/// chance matches in a changing picture say. On the plan's captures a
/// scroll's vector has had at least one anchor in 160 and video's likeliest
/// chance match one in 900.
const FRAME_VOTES: usize = 4;
const FRAME_SHARE: usize = 256;

/// The multipliers of the polynomial hashes across a row and down a column.
const ACROSS: u32 = 0x9e37_79b1;
const DOWN: u32 = 0x85eb_ca77;

/// A hash that is already a hash, kept as it is.
#[derive(Default)]
struct Identity(u64);

impl Hasher for Identity {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _: &[u8]) {
        unreachable!("only u32 keys")
    }
    fn write_u32(&mut self, v: u32) {
        self.0 = u64::from(v);
    }
}

/// Where the frame before has anchors of one hash, as (x, y): how many, and
/// the first of them; more than `MAX_REPEATS` once it has too many.
type Places = (usize, [(u16, u16); MAX_REPEATS]);

type Table = HashMap<u32, Places, BuildHasherDefault<Identity>>;

/// An 8×8's anchors: each one's hash and where in the 8×8 it starts,
/// `8 * y + x`.
type Anchors = [(u32, u8); PER_8X8];

/// The frame's measures the search works in.
struct Shape {
    width: usize,
    height: usize,
    mi_cols: usize,
    mi_rows: usize,
    sb_cols: usize,
    sb_rows: usize,
}

/// What one thread hashes into: the row hashes of a row of 8×8s and the
/// window hashes taken from them.
#[derive(Default)]
struct Scratch {
    rows: Vec<u32>,
    windows: Vec<u32>,
}

/// The search's memory of the source coded last.
pub(super) struct Search {
    shape: Shape,
    threads: usize,
    /// Per 8×8, the anchors whose windows start in it, and how many.
    anchors: Vec<Anchors>,
    counts: Vec<u8>,
    table: Table,
    /// Per thread.
    scratch: Vec<Scratch>,
}

impl Search {
    /// The search of a frame of `geometry` on up to `threads` threads.
    pub(super) fn new(geometry: &Geometry, threads: usize) -> Self {
        let mis = geometry.mi_rows * geometry.mi_cols;
        Self {
            shape: Shape {
                width: usize::from(geometry.width),
                height: usize::from(geometry.height),
                mi_cols: geometry.mi_cols,
                mi_rows: geometry.mi_rows,
                sb_cols: geometry.sb_cols,
                sb_rows: geometry.sb_rows,
            },
            threads,
            anchors: vec![[(0, 0); PER_8X8]; mis],
            counts: vec![0; mis],
            table: HashMap::default(),
            scratch: (0..threads).map(|_| Scratch::default()).collect(),
        }
    }

    /// Take the anchors of `luma`, the source being coded at the picture's
    /// width, where it differs from the source coded last: in every 8×8 not
    /// marked in `unchanged`, or all of them when it is `None`. In an inter
    /// frame (`inter`), give back per superblock in raster order the vectors
    /// it offers, the zero vector where it has fewer than `VECTORS`.
    pub(super) fn update(&mut self, luma: &[u8], unchanged: Option<&[bool]>, inter: bool) -> Vec<[Mv; VECTORS]> {
        let shape = &self.shape;
        let mi_cols = shape.mi_cols;
        let changed = |r: usize, c: usize| r < shape.mi_rows && c < mi_cols && !unchanged.is_some_and(|u| u[r * mi_cols + c]);
        // An 8×8's anchors are taken again when a window starting in it
        // reaches a changed pixel: one in it, or right of it, or below.
        let stale: Vec<bool> = (0..shape.mi_rows * mi_cols)
            .map(|i| {
                let (r, c) = (i / mi_cols, i % mi_cols);
                changed(r, c) || changed(r, c + 1) || changed(r + 1, c) || changed(r + 1, c + 1)
            })
            .collect();
        if inter && stale.iter().any(|&s| s) {
            self.index();
        }
        let shape = &self.shape;
        let table = inter.then_some(&self.table);
        let band = shape.mi_rows.div_ceil(self.threads) * mi_cols;
        let bands = self.anchors.chunks_mut(band).zip(self.counts.chunks_mut(band)).zip(stale.chunks(band)).zip(&mut self.scratch).enumerate();
        let found = on_threads(self.threads, bands, |(b, (((anchors, counts), stale), scratch))| {
            let mut found = Vec::new();
            let mut looked_up = 0;
            let first = b * band;
            for (k, stale) in stale.chunks(mi_cols).enumerate() {
                let r = first / mi_cols + k;
                let row = k * mi_cols..(k + 1) * mi_cols;
                let (anchors, counts) = (&mut anchors[row.clone()], &mut counts[row]);
                let mut c = 0;
                while c < mi_cols {
                    if !stale[c] {
                        c += 1;
                        continue;
                    }
                    let start = c;
                    while c < mi_cols && stale[c] {
                        c += 1;
                    }
                    shape.hash_run(luma, r, start..c, scratch, anchors, counts);
                    let Some(table) = table else { continue };
                    for i in start..c {
                        for &a in &anchors[i][..usize::from(counts[i])] {
                            let (x, y) = shape.place(r * mi_cols + i, a);
                            looked_up += 1;
                            if let Some(&(count, places)) = table.get(&a.0).filter(|p| p.0 <= MAX_REPEATS) {
                                for &(px, py) in &places[..count] {
                                    let mv = (i32::from(py) - y as i32, i32::from(px) - x as i32);
                                    if mv != (0, 0) && mv.0.abs() <= MAX_PIXELS && mv.1.abs() <= MAX_PIXELS {
                                        found.push(((y / 64) * shape.sb_cols + x / 64, Mv::pixels(mv.0, mv.1)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            (found, looked_up)
        });
        if !inter {
            return Vec::new();
        }
        let looked_up: usize = found.iter().map(|f| f.1).sum();
        let mut found: Vec<(usize, Mv)> = found.into_iter().flat_map(|f| f.0).collect();
        let frame = most(found.iter().map(|&(_, mv)| mv).collect(), FRAME_VOTES.max(looked_up / FRAME_SHARE), 2);
        found.sort_unstable();
        let mut vectors = vec![[Mv::ZERO; VECTORS]; shape.sb_rows * shape.sb_cols];
        for votes in found.chunk_by(|a, b| a.0 == b.0) {
            let own = most(votes.iter().map(|&(_, mv)| mv).collect(), OWN_VOTES, 2);
            vectors[votes[0].0][..own.len()].copy_from_slice(&own);
        }
        for v in &mut vectors {
            let mut at = v.iter().position(|&mv| mv == Mv::ZERO).unwrap_or(VECTORS);
            for &mv in &frame {
                if at < VECTORS && !v.contains(&mv) {
                    v[at] = mv;
                    at += 1;
                }
            }
        }
        vectors
    }

    /// Index the anchors kept, which are the source coded last's, by hash.
    fn index(&mut self) {
        self.table.clear();
        for i in 0..self.anchors.len() {
            for &a in &self.anchors[i][..usize::from(self.counts[i])] {
                let (x, y) = self.shape.place(i, a);
                let (count, places) = self.table.entry(a.0).or_insert((0, [(0, 0); MAX_REPEATS]));
                if *count < MAX_REPEATS {
                    places[*count] = (x as u16, y as u16);
                }
                *count = (*count + 1).min(MAX_REPEATS + 1);
            }
        }
    }
}

impl Shape {
    /// Where in the frame an anchor kept in 8×8 `i` starts.
    fn place(&self, i: usize, (_, offset): (u32, u8)) -> (usize, usize) {
        let offset = usize::from(offset);
        ((i % self.mi_cols) * 8 + offset % 8, (i / self.mi_cols) * 8 + offset / 8)
    }

    /// Take again the anchors of the 8×8s `cols` of 8×8 row `r` from `luma`,
    /// into `anchors` and `counts`, that row's.
    fn hash_run(&self, luma: &[u8], r: usize, cols: std::ops::Range<usize>, scratch: &mut Scratch, anchors: &mut [Anchors], counts: &mut [u8]) {
        let (width, height) = (self.width, self.height);
        counts[cols.clone()].fill(0);
        // Windows start where a window one pixel right and one down still
        // fits, so each can be held to its neighbours.
        let (x0, y0) = (cols.start * 8, r * 8);
        if width < 9 || height < 9 || x0 > width - 9 || y0 > height - 9 {
            return;
        }
        let x1 = (cols.end * 8).min(width - 8);
        let y1 = (y0 + 8).min(height - 8);
        // Row hashes of the 8 pixels from each x in x0..=x1, rows y0..y1 + 8.
        let across = x1 + 1 - x0;
        let row_count = y1 + 8 - y0;
        let Scratch { rows, windows } = scratch;
        rows.clear();
        rows.resize(across * row_count, 0);
        let top = ACROSS.wrapping_pow(8);
        for (k, row) in rows.chunks_exact_mut(across).enumerate() {
            let pixels = &luma[(y0 + k) * width..][..width];
            let mut h = pixels[x0..x0 + 8].iter().fold(0u32, |h, &p| h.wrapping_mul(ACROSS).wrapping_add(u32::from(p)));
            row[0] = h;
            for (j, out) in row.iter_mut().enumerate().skip(1) {
                let x = x0 + j - 1;
                h = h.wrapping_mul(ACROSS).wrapping_add(u32::from(pixels[x + 8])).wrapping_sub(top.wrapping_mul(u32::from(pixels[x])));
                *out = h;
            }
        }
        // Window hashes at y0..=y1 from the row hashes, rolled down each
        // column as the row hashes are rolled across.
        let down = y1 + 1 - y0;
        windows.clear();
        windows.resize(across * down, 0);
        for (j, w) in windows[..across].iter_mut().enumerate() {
            *w = (0..8).fold(0u32, |h, d| h.wrapping_mul(DOWN).wrapping_add(rows[d * across + j]));
        }
        let top = DOWN.wrapping_pow(8);
        for k in 1..down {
            let (done, next) = windows.split_at_mut(k * across);
            let above = &done[(k - 1) * across..];
            let (leaving, entering) = (&rows[(k - 1) * across..][..across], &rows[(k + 7) * across..][..across]);
            for (j, w) in next[..across].iter_mut().enumerate() {
                *w = above[j].wrapping_mul(DOWN).wrapping_add(entering[j]).wrapping_sub(top.wrapping_mul(leaving[j]));
            }
        }
        for y in y0..y1 {
            for x in x0..x1 {
                let (j, k) = (x - x0, y - y0);
                let h = windows[k * across + j];
                if h.wrapping_mul(SCRAMBLE) >> (32 - ANCHOR_BITS) != 0 || h == windows[k * across + j + 1] || h == windows[(k + 1) * across + j] {
                    continue;
                }
                let c = x / 8;
                let n = usize::from(counts[c]);
                if n < PER_8X8 {
                    anchors[c][n] = (h, ((y % 8) * 8 + x % 8) as u8);
                    counts[c] += 1;
                }
            }
        }
    }
}

/// The `count` vectors of `votes` found most often, at least `least` times
/// each, the most first.
fn most(mut votes: Vec<Mv>, least: usize, count: usize) -> Vec<Mv> {
    votes.sort_unstable();
    let mut runs: Vec<(usize, Mv)> = votes.chunk_by(|a, b| a == b).filter(|run| run.len() >= least).map(|run| (run.len(), run[0])).collect();
    runs.sort_unstable_by(|a, b| b.cmp(a));
    runs.into_iter().take(count).map(|(_, mv)| mv).collect()
}
