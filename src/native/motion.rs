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
//! taken and looked up in a band of 8×8 rows on each thread. The table of
//! the frame before's anchors by hash is kept too, and brought up to date
//! with the 8×8s whose anchors were taken again, unless so many were that
//! filling it again is cheaper.
//!
//! What moves but does not arrive whole, which a hash cannot match, each
//! superblock that changed finds by its projections (libvpx's
//! `vp9_int_pro_motion_estimation`): its rows' sums matched against the
//! frame before's down the superblock's column, and then its columns' sums
//! across the rows that matched. Text scrolled in a picture coded with loss
//! is never the same twice, and that is how the plan's captures have it.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use super::mv::{MAX_PIXELS, Mv};
use super::pool::Pool;
use super::{Geometry, Reconstruction, on_threads, on_workers};

/// The vectors a superblock offers: its own anchors' best two, then its
/// projections', then the frame's best two.
pub(super) const VECTORS: usize = 5;

/// How far each way, in whole pixels, a superblock's projections are
/// matched. libvpx matches half a superblock, which misses a terminal
/// scrolling two of its rows, 38 pixels; on the plan's captures twice this
/// found next to nothing more.
const REACH: usize = 64;

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

/// A hash that is already a hash, spread over the 64 bits a table reads:
/// it takes its slot from the low ones and tells the keys of a slot's
/// group apart by the top seven, which of a hash kept as it is are zero.
#[derive(Default)]
struct Spread(u64);

impl Hasher for Spread {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _: &[u8]) {
        unreachable!("only u32 keys")
    }
    fn write_u32(&mut self, v: u32) {
        self.0 = u64::from(v).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
}

/// Where the frame before has anchors of one hash, as (x, y): how many, and
/// each of them while there are no more than `MAX_REPEATS`. Once there have
/// been more the places are not kept, and the hash says nothing until the
/// table is filled again.
#[derive(Clone, Copy, Default)]
struct Places {
    count: u32,
    spoiled: bool,
    places: [(u16, u16); MAX_REPEATS],
}

impl Places {
    /// The places, where the hash has few enough anchors to say where
    /// something came from.
    fn usable(&self) -> Option<&[(u16, u16)]> {
        (!self.spoiled && self.count as usize <= MAX_REPEATS).then(|| &self.places[..self.count as usize])
    }
}

type Table = HashMap<u32, Places, BuildHasherDefault<Spread>>;

/// Add an anchor of `hash` at `place` to `table`.
fn insert(table: &mut Table, hash: u32, place: (u16, u16)) {
    let p = table.entry(hash).or_default();
    if p.count as usize >= MAX_REPEATS {
        p.spoiled = true;
    } else {
        p.places[p.count as usize] = place;
    }
    p.count += 1;
}

/// Take the anchor of `hash` at `place` out of `table`, which holds it.
fn remove(table: &mut Table, hash: u32, place: (u16, u16)) {
    let p = table.get_mut(&hash).expect("an anchor indexed is in the table");
    p.count -= 1;
    if let Some(at) = (!p.spoiled).then(|| p.places[..=p.count as usize].iter().position(|&q| q == place)).flatten() {
        p.places[at] = p.places[p.count as usize];
    }
    if p.count == 0 {
        table.remove(&hash);
    }
}

/// The share of a frame's 8×8s, as `1 / REFILL`, above which a table is
/// filled again rather than brought up to date one anchor at a time.
const REFILL: usize = 4;

/// Which of the tables, one per thread so that each thread fills one, holds
/// the anchors whose hash is `hash`: its top sixteen bits scaled to the
/// tables, every thread asking of every anchor.
fn shard(hash: u32, shards: usize) -> usize {
    ((hash >> 16) as usize * shards) >> 16
}

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
/// window hashes taken from them; and where it copies the frame before out
/// to project a superblock against it.
#[derive(Default)]
struct Scratch {
    rows: Vec<u32>,
    windows: Vec<u32>,
    /// A row's hashes of two pixels and of four, and its windows' flags.
    pairs: Vec<u32>,
    fours: Vec<u32>,
    flags: Vec<u8>,
    patch: Vec<u8>,
}

/// The search's memory of the source coded last.
pub(super) struct Search {
    shape: Shape,
    /// Per 8×8, the anchors whose windows start in it, and how many.
    anchors: Vec<Anchors>,
    counts: Vec<u8>,
    /// The anchors of the source coded last by hash, in a table per thread
    /// (`shard`).
    tables: Vec<Table>,
    /// Whether the tables have to be filled from every anchor: before any
    /// frame, after a keyframe, and after a frame that took the anchors of
    /// more than one 8×8 in `REFILL` again.
    refill: bool,
    /// Whether the anchors have to be taken of the whole source coded
    /// last: after a keyframe, which takes none.
    unhashed: bool,
    /// The 8×8s whose anchors were taken again last, with the anchors they
    /// had, which the tables still hold.
    pending: Vec<(usize, Anchors, u8)>,
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
            anchors: vec![[(0, 0); PER_8X8]; mis],
            counts: vec![0; mis],
            tables: (0..threads).map(|_| HashMap::default()).collect(),
            refill: true,
            unhashed: true,
            pending: Vec::new(),
            scratch: (0..threads).map(|_| Scratch::default()).collect(),
        }
    }

    /// A keyframe was coded: it reads nothing of the search, and its
    /// anchors are taken by the inter frame after it, if one follows, so
    /// that the longest frame of a stream does not wait on them.
    pub(super) fn forget(&mut self) {
        self.unhashed = true;
        self.refill = true;
        self.pending.clear();
    }

    /// Whether the anchors of the source coded last are still to be taken.
    pub(super) fn waiting(&self) -> bool {
        self.unhashed
    }

    /// Before an inter frame that changes the picture, with `luma` still
    /// the source coded last at the picture's width: take every anchor of
    /// it, none being taken since a keyframe.
    pub(super) fn catch_up(&mut self, pool: &Pool, luma: &[u8]) {
        if !std::mem::take(&mut self.unhashed) {
            return;
        }
        let shape = &self.shape;
        let mi_cols = shape.mi_cols;
        let rows = self.anchors.chunks_mut(mi_cols).zip(self.counts.chunks_mut(mi_cols)).enumerate();
        on_workers(pool, rows, &mut self.scratch, |(r, (anchors, counts)), scratch| shape.hash_run(luma, r, 0..mi_cols, scratch, anchors, counts));
    }

    /// Take the anchors of `luma`, the source being coded at the picture's
    /// width, where it differs from the source coded last, whose anchors
    /// are kept: in every 8×8 not marked in `unchanged`. Give back per
    /// superblock in raster order the vectors it offers for a frame coded
    /// against `reference`, the zero vector where it has fewer than
    /// `VECTORS`.
    pub(super) fn update(&mut self, pool: &Pool, threads: usize, luma: &[u8], unchanged: &[bool], reference: &Reconstruction) -> Vec<[Mv; VECTORS]> {
        let shape = &self.shape;
        let mi_cols = shape.mi_cols;
        // An 8×8's anchors are taken again when a window starting in it
        // reaches a changed pixel: one in it, or right of it, or below. A
        // row of 8×8s with no change in it or the row below has none.
        let row_changed: Vec<bool> = (0..shape.mi_rows).map(|r| unchanged[r * mi_cols..][..mi_cols].contains(&false)).collect();
        // A frame that changes nothing searches nothing, and leaves the
        // tables and the anchors as they are, taken or not.
        if !row_changed.contains(&true) {
            return vec![[Mv::ZERO; VECTORS]; shape.sb_rows * shape.sb_cols];
        }
        let mut stale = vec![false; shape.mi_rows * mi_cols];
        for (r, stale) in stale.chunks_exact_mut(mi_cols).enumerate() {
            let below = row_changed.get(r + 1).is_some_and(|&n| n);
            if !row_changed[r] && !below {
                continue;
            }
            // The row's 8×8s and those under them, or the row again under
            // the last, which adds nothing.
            let here = &unchanged[r * mi_cols..][..mi_cols];
            let under = if below { &unchanged[(r + 1) * mi_cols..][..mi_cols] } else { here };
            for ((stale, here), under) in stale.iter_mut().zip(here.windows(2)).zip(under.windows(2)) {
                *stale = !(here[0] & here[1] & under[0] & under[1]);
            }
            stale[mi_cols - 1] = !(here[mi_cols - 1] & under[mi_cols - 1]);
        }
        // The tables are brought to the source coded last, which the
        // anchors are of, before any of them is taken again.
        self.bring_to_date(pool, threads);
        // Where so many 8×8s are taken again that filling the tables is
        // cheaper than bringing them up to date, what they held is not
        // kept.
        self.refill = stale.iter().filter(|&&stale| stale).count() * REFILL > self.anchors.len();
        let keep = !self.refill;
        let shape = &self.shape;
        let tables = &self.tables[..];
        // A row of 8×8s at a time, as each thread is free for one: what
        // changed lies in some rows and not others.
        let rows = self.anchors.chunks_mut(mi_cols).zip(self.counts.chunks_mut(mi_cols)).zip(stale.chunks(mi_cols)).enumerate();
        let found = on_workers(pool, rows, &mut self.scratch[..threads], |(r, ((anchors, counts), stale)), scratch| {
            let mut found = Vec::new();
            let mut replaced = Vec::new();
            let mut looked_up = 0;
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
                if keep {
                    replaced.extend((start..c).map(|i| (r * mi_cols + i, anchors[i], counts[i])));
                }
                shape.hash_run(luma, r, start..c, scratch, anchors, counts);
                for i in start..c {
                    for &a in &anchors[i][..usize::from(counts[i])] {
                        let (x, y) = shape.place(r * mi_cols + i, a);
                        looked_up += 1;
                        if let Some(places) = tables[shard(a.0, tables.len())].get(&a.0).and_then(Places::usable) {
                            for &(px, py) in places {
                                let mv = (i32::from(py) - y as i32, i32::from(px) - x as i32);
                                if mv != (0, 0) && mv.0.abs() <= MAX_PIXELS && mv.1.abs() <= MAX_PIXELS {
                                    found.push(((y / 64) * shape.sb_cols + x / 64, Mv::pixels(mv.0, mv.1)));
                                }
                            }
                        }
                    }
                }
            }
            (found, looked_up, replaced)
        });
        self.pending = found.iter().flat_map(|f| f.2.iter().copied()).collect();
        // Per superblock, its projections' vector where it changed, a row
        // of superblocks at a time.
        let projected = on_workers(pool, 0..shape.sb_rows, &mut self.scratch[..threads], |sb_row, scratch| {
            (0..shape.sb_cols)
                .map(|sb_col| {
                    let cols = sb_col * 8..(sb_col * 8 + 8).min(mi_cols);
                    let moved = (sb_row * 8..(sb_row * 8 + 8).min(shape.mi_rows)).any(|r| unchanged[r * mi_cols..][cols.clone()].contains(&false));
                    if moved { shape.project(luma, reference, (sb_row, sb_col), &mut scratch.patch) } else { Mv::ZERO }
                })
                .collect::<Vec<_>>()
        });
        let looked_up: usize = found.iter().map(|f| f.1).sum();
        let mut found: Vec<(usize, Mv)> = found.into_iter().flat_map(|f| f.0).collect();
        let frame = most(found.iter().map(|&(_, mv)| mv).collect(), FRAME_VOTES.max(looked_up / FRAME_SHARE), 2);
        found.sort_unstable();
        let mut vectors = vec![[Mv::ZERO; VECTORS]; shape.sb_rows * shape.sb_cols];
        for votes in found.chunk_by(|a, b| a.0 == b.0) {
            let own = most(votes.iter().map(|&(_, mv)| mv).collect(), OWN_VOTES, 2);
            vectors[votes[0].0][..own.len()].copy_from_slice(&own);
        }
        for (v, projected) in vectors.iter_mut().zip(projected.into_iter().flatten()) {
            let mut at = v.iter().position(|&mv| mv == Mv::ZERO).unwrap_or(VECTORS);
            for &mv in std::iter::once(&projected).chain(&frame) {
                if at < VECTORS && !v.contains(&mv) {
                    v[at] = mv;
                    at += 1;
                }
            }
        }
        vectors
    }

    /// Bring the tables to the anchors kept, which are the source coded
    /// last's: filled again, or brought up to date.
    fn bring_to_date(&mut self, pool: &Pool, threads: usize) {
        if self.refill {
            self.index(pool);
        } else {
            self.bring_up(pool, threads);
        }
        self.pending.clear();
    }

    /// Bring the tables to the anchors kept, which are the source coded
    /// last's, from the 8×8s whose anchors were taken again last: each
    /// thread takes the anchors those had out of its table and puts in the
    /// ones they have.
    fn bring_up(&mut self, pool: &Pool, threads: usize) {
        let Self { shape, anchors, counts, tables, pending, .. } = self;
        let shards = tables.len();
        on_threads(pool, threads, tables.iter_mut().enumerate(), |(s, table)| {
            for &(i, old, n) in pending.iter() {
                for &a in old[..usize::from(n)].iter().filter(|a| shard(a.0, shards) == s) {
                    let (x, y) = shape.place(i, a);
                    remove(table, a.0, (x as u16, y as u16));
                }
            }
            for &(i, ..) in pending.iter() {
                for &a in anchors[i][..usize::from(counts[i])].iter().filter(|a| shard(a.0, shards) == s) {
                    let (x, y) = shape.place(i, a);
                    insert(table, a.0, (x as u16, y as u16));
                }
            }
        });
    }

    /// Index the anchors kept, which are the source coded last's, by hash,
    /// each thread filling a table.
    fn index(&mut self, pool: &Pool) {
        let Self { shape, anchors, counts, tables, .. } = self;
        let shards = tables.len();
        // Each thread reads every anchor and takes its table's, in order.
        on_threads(pool, shards, tables.iter_mut().enumerate(), |(s, table)| {
            table.clear();
            for (i, (anchors, &count)) in anchors.iter().zip(counts.iter()).enumerate() {
                for &a in anchors[..usize::from(count)].iter().filter(|a| shard(a.0, shards) == s) {
                    let (x, y) = shape.place(i, a);
                    insert(table, a.0, (x as u16, y as u16));
                }
            }
        });
    }
}

impl Shape {
    /// The vector of whole pixels, at most `REACH` each way, from the
    /// superblock at `(sb_row, sb_col)` of `luma` to where `reference` best
    /// matches its rows' sums, and then its columns' sums in the rows that
    /// matched; zero for a superblock the picture does not hold whole.
    fn project(&self, luma: &[u8], reference: &Reconstruction, (sb_row, sb_col): (usize, usize), patch: &mut Vec<u8>) -> Mv {
        let (x, y) = (sb_col * 64, sb_row * 64);
        if x + 64 > self.width || y + 64 > self.height {
            return Mv::ZERO;
        }
        let (mut rows, mut cols) = ([0u32; 64], [0u32; 64]);
        for (r, sum) in rows.iter_mut().enumerate() {
            let row = &luma[(y + r) * self.width + x..][..64];
            *sum = row.iter().map(|&v| u32::from(v)).sum();
            for (c, &v) in cols.iter_mut().zip(row) {
                *c += u32::from(v);
            }
        }
        let frame = (self.width, self.height);
        let reach = REACH as isize;
        let mut sums = [0u32; 64 + 2 * REACH];
        let (strip, stride) = reference.block(0, (x as isize, y as isize - reach), (64, sums.len()), frame, patch);
        for (r, sum) in sums.iter_mut().enumerate() {
            *sum = strip[r * stride..][..64].iter().map(|&v| u32::from(v)).sum();
        }
        let down = offset(&rows, &sums);
        let (strip, stride) = reference.block(0, (x as isize - reach, y as isize + down), (sums.len(), 64), frame, patch);
        sums.fill(0);
        for row in strip.chunks(stride).take(64) {
            for (sum, &v) in sums.iter_mut().zip(row) {
                *sum += u32::from(v);
            }
        }
        let across = offset(&cols, &sums);
        Mv::pixels(down as i32, across as i32)
    }

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
        let Scratch { rows, windows, pairs, fours, flags, .. } = scratch;
        // Every value of these is written before it is read: nothing is
        // cleared but the first windows, which are summed into.
        rows.resize(across * row_count, 0);
        // Each the polynomial of its eight pixels, made of two of four pixels,
        // which are each made of two of two: three multiplies a hash where
        // a term at a time is eight, and every position's at once.
        let [by2, by4] = [ACROSS.wrapping_pow(2), ACROSS.wrapping_pow(4)];
        pairs.resize(across + 6, 0);
        fours.resize(across + 4, 0);
        for (k, row) in rows.chunks_exact_mut(across).enumerate() {
            let pixels = &luma[(y0 + k) * width + x0..][..across + 7];
            for ((out, &a), &b) in pairs.iter_mut().zip(pixels).zip(&pixels[1..]) {
                *out = ACROSS.wrapping_mul(u32::from(a)).wrapping_add(u32::from(b));
            }
            for ((out, &a), &b) in fours.iter_mut().zip(&pairs[..]).zip(&pairs[2..]) {
                *out = by2.wrapping_mul(a).wrapping_add(b);
            }
            for ((out, &a), &b) in row.iter_mut().zip(&fours[..]).zip(&fours[4..]) {
                *out = by4.wrapping_mul(a).wrapping_add(b);
            }
        }
        // Window hashes at y0..=y1 from the row hashes, rolled down each
        // column as the row hashes are rolled across.
        let down = y1 + 1 - y0;
        windows.resize(across * down, 0);
        windows[..across].fill(0);
        for row in rows.chunks_exact(across).take(8) {
            for (w, &h) in windows[..across].iter_mut().zip(row) {
                *w = w.wrapping_mul(DOWN).wrapping_add(h);
            }
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
        // Which windows are anchors, a flag each for the whole row at once,
        // and then only the few that are, found eight flags at a time.
        let n = x1 - x0;
        flags.clear();
        flags.resize(n.next_multiple_of(8), 0);
        for (k, y) in (y0..y1).enumerate() {
            let (row, below) = (&windows[k * across..][..across], &windows[(k + 1) * across..][..n]);
            for (((flag, &h), &right), &below) in flags.iter_mut().zip(row).zip(&row[1..]).zip(below) {
                *flag = u8::from((h.wrapping_mul(SCRAMBLE) >> (32 - ANCHOR_BITS) == 0) & (h != right) & (h != below));
            }
            for (e, eight) in flags.as_chunks::<8>().0.iter().enumerate() {
                let mut set = u64::from_le_bytes(*eight);
                while set != 0 {
                    let j = e * 8 + set.trailing_zeros() as usize / 8;
                    set &= set - 1;
                    let x = x0 + j;
                    let c = x / 8;
                    let count = usize::from(counts[c]);
                    if count < PER_8X8 {
                        anchors[c][count] = (row[j], ((y % 8) * 8 + x % 8) as u8);
                        counts[c] += 1;
                    }
                }
            }
        }
    }
}

/// Where in `sums` the run as long as `block` differs least from it, the
/// nearer the middle the better among equals, as an offset from the middle.
fn offset(block: &[u32; 64], sums: &[u32; 64 + 2 * REACH]) -> isize {
    let at = (0..=2 * REACH)
        .min_by_key(|&o| (block.iter().zip(&sums[o..]).map(|(&a, &b)| a.abs_diff(b)).sum::<u32>(), o.abs_diff(REACH)))
        .expect("an offset");
    at as isize - REACH as isize
}

/// The `count` vectors of `votes` found most often, at least `least` times
/// each, the most first.
fn most(mut votes: Vec<Mv>, least: usize, count: usize) -> Vec<Mv> {
    votes.sort_unstable();
    let mut runs: Vec<(usize, Mv)> = votes.chunk_by(|a, b| a == b).filter(|run| run.len() >= least).map(|run| (run.len(), run[0])).collect();
    runs.sort_unstable_by(|a, b| b.cmp(a));
    runs.into_iter().take(count).map(|(_, mv)| mv).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `w`×`h` of noise from `seed`, which has anchors everywhere.
    fn noise(w: usize, h: usize, seed: u32) -> Vec<u8> {
        let mut state = seed;
        (0..w * h)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect()
    }

    /// Every usable place in `tables`, by hash, sorted.
    fn usable(tables: &[Table]) -> Vec<(u32, Vec<(u16, u16)>)> {
        let mut all: Vec<_> = tables
            .iter()
            .flat_map(|t| t.iter())
            .filter_map(|(&h, p)| {
                p.usable().map(|places| {
                    let mut places = places.to_vec();
                    places.sort_unstable();
                    (h, places)
                })
            })
            .collect();
        all.sort_unstable();
        all
    }

    /// The anchors of a frame are those a window at a time gives, its hash
    /// the polynomial of its pixels a term at a time: in noise, where hashes
    /// are anchors by chance, in a picture of few values, where windows are
    /// the same as their neighbours, and at widths that end inside an 8×8.
    #[test]
    fn the_anchors_are_what_a_window_at_a_time_gives() {
        for (width, height, seed) in [(64, 64, 1), (77, 43, 2), (129, 70, 3), (200, 33, 4), (16, 9, 5), (9, 16, 6)] {
            let mut luma = noise(width, height, seed);
            if seed % 2 == 0 {
                luma.iter_mut().for_each(|v| *v = if *v < 200 { 16 } else { *v & 1 });
            }
            let shape = Shape { width, height, mi_cols: width.div_ceil(8), mi_rows: height.div_ceil(8), sb_cols: width.div_ceil(64), sb_rows: height.div_ceil(64) };
            let window = |x: usize, y: usize| {
                (0..8).fold(0u32, |h, d| {
                    let row = luma[(y + d) * width + x..][..8].iter().fold(0u32, |h, &v| h.wrapping_mul(ACROSS).wrapping_add(u32::from(v)));
                    h.wrapping_mul(DOWN).wrapping_add(row)
                })
            };
            let mut scratch = Scratch::default();
            for r in 0..shape.mi_rows {
                let mut expected = vec![(Anchors::default(), 0u8); shape.mi_cols];
                for y in r * 8..(r * 8 + 8).min(height.saturating_sub(8)) {
                    for x in 0..width - 8 {
                        let h = window(x, y);
                        let (kept, count) = &mut expected[x / 8];
                        if h.wrapping_mul(SCRAMBLE) >> (32 - ANCHOR_BITS) == 0 && h != window(x + 1, y) && h != window(x, y + 1) && usize::from(*count) < PER_8X8 {
                            kept[usize::from(*count)] = (h, ((y % 8) * 8 + x % 8) as u8);
                            *count += 1;
                        }
                    }
                }
                // In one run, and in two that meet inside the row.
                for split in [0, shape.mi_cols / 2] {
                    let (mut anchors, mut counts) = (vec![Anchors::default(); shape.mi_cols], vec![9u8; shape.mi_cols]);
                    shape.hash_run(&luma, r, 0..split, &mut scratch, &mut anchors, &mut counts);
                    shape.hash_run(&luma, r, split..shape.mi_cols, &mut scratch, &mut anchors, &mut counts);
                    for (c, (kept, count)) in expected.iter().enumerate() {
                        let n = usize::from(*count);
                        assert_eq!((counts[c], &anchors[c][..n]), (*count, &kept[..n]), "{width}x{height}, row {r}, 8x8 {c}, split at {split}");
                    }
                }
            }
        }
    }

    /// The time one thread takes over the anchors of a 1920×1080 frame, of
    /// noise and of a picture of few values. A measurement, not a check; run
    /// it in release:
    ///
    /// ```text
    /// cargo test --release --features native anchor_time -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a measurement that wants a release build"]
    fn anchor_time() {
        let (width, height) = (1920, 1080);
        let shape = Shape { width, height, mi_cols: width / 8, mi_rows: height / 8, sb_cols: width / 64, sb_rows: height.div_ceil(64) };
        for (name, few) in [("noise", false), ("few values", true)] {
            let mut luma = noise(width, height, 7);
            if few {
                luma.iter_mut().for_each(|v| *v = if *v < 200 { 16 } else { *v & 1 });
            }
            let (mut anchors, mut counts) = (vec![Anchors::default(); shape.mi_cols], vec![0u8; shape.mi_cols]);
            let mut scratch = Scratch::default();
            let least = (0..20)
                .map(|_| {
                    let started = std::time::Instant::now();
                    for r in 0..shape.mi_rows {
                        shape.hash_run(&luma, r, 0..shape.mi_cols, &mut scratch, &mut anchors, &mut counts);
                    }
                    started.elapsed().as_secs_f64() * 1000.0
                })
                .fold(f64::INFINITY, f64::min);
            println!("{name}: {least:.2} ms");
        }
    }

    /// A frame that changes nothing takes no anchor and searches nothing,
    /// and the frame after it finds what it would have found without it.
    #[test]
    fn a_still_frame_leaves_the_anchors_to_the_frame_that_changes() {
        let (w, h) = (320usize, 200usize);
        let geometry = Geometry::new(w as u16, h as u16);
        let pool = Pool::new(3);
        let recon = Reconstruction::new(&geometry, 0);
        let luma = noise(w, h, 1);
        let mut scrolled = luma.clone();
        scrolled.rotate_left(w * 16);
        let count = geometry.mi_rows * geometry.mi_cols;
        let mut direct = Search::new(&geometry, 3);
        direct.catch_up(&pool, &luma);
        let expected = direct.update(&pool, 3, &scrolled, &vec![false; count], &recon);
        assert!(expected.iter().any(|v| v[0] != Mv::ZERO), "the scroll is found");
        let mut search = Search::new(&geometry, 3);
        let none = search.update(&pool, 3, &luma, &vec![true; count], &recon);
        assert!(none.iter().all(|v| v.iter().all(|&mv| mv == Mv::ZERO)));
        assert!(search.waiting());
        search.catch_up(&pool, &luma);
        assert!(search.update(&pool, 3, &scrolled, &vec![false; count], &recon) == expected);
    }

    /// After frames that change a little and frames that change much, the
    /// tables brought up to date hold what tables filled from every anchor
    /// hold, but for hashes that repeated too much at some point.
    #[test]
    fn the_tables_brought_up_to_date_are_the_tables_filled_again() {
        let (w, h) = (320usize, 200usize);
        let geometry = Geometry::new(w as u16, h as u16);
        let pool = Pool::new(3);
        let mut search = Search::new(&geometry, 3);
        let recon = Reconstruction::new(&geometry, 0);
        let mut luma = noise(w, h, 1);
        let all_changed = vec![false; geometry.mi_rows * geometry.mi_cols];
        search.catch_up(&pool, &luma);
        let mut unchanged = all_changed.clone();
        for frame in 1..12usize {
            // A patch of new noise, or a scroll, or everything new.
            let mut next = luma.clone();
            match frame % 4 {
                3 => next = noise(w, h, frame as u32),
                2 => next.rotate_left(w * 7),
                _ => {
                    let (x0, y0) = (frame * 13 % (w - 40), frame * 29 % (h - 24));
                    let patch = noise(40, 24, 100 + frame as u32);
                    for r in 0..24 {
                        next[(y0 + r) * w + x0..][..40].copy_from_slice(&patch[r * 40..][..40]);
                    }
                }
            }
            for (i, u) in unchanged.iter_mut().enumerate() {
                let (r, c) = (i / geometry.mi_cols, i % geometry.mi_cols);
                *u = (0..8).all(|y| luma[(r * 8 + y) * w + c * 8..][..8] == next[(r * 8 + y) * w + c * 8..][..8]);
            }
            search.update(&pool, 3, &next, &unchanged, &recon);
            luma = next;
        }
        // The tables are now of the frame before `luma`; refill from the
        // same anchors and compare.
        let mut refilled = Search::new(&geometry, 3);
        refilled.anchors = search.anchors.clone();
        refilled.counts = search.counts.clone();
        search.bring_to_date(&pool, 3);
        refilled.index(&pool);
        let (kept, fresh) = (usable(&search.tables), usable(&refilled.tables));
        assert!(!kept.is_empty());
        let spoiled: Vec<_> = fresh.iter().filter(|f| !kept.contains(f)).collect();
        for f in &spoiled {
            let p = search.tables[shard(f.0, 3)].get(&f.0).expect("a hash the fresh tables have is in the kept ones");
            assert!(p.spoiled, "{:?} is in the fresh tables and not the kept ones", f);
            assert_eq!(p.count as usize, f.1.len());
        }
        assert!(kept.iter().all(|k| fresh.contains(k)), "the kept tables hold a place the fresh ones do not");
        assert!(spoiled.len() * 10 < fresh.len(), "{} of {} hashes spoiled", spoiled.len(), fresh.len());
    }
}
