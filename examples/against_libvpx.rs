//! A program of its own for the three measurements of the Rust coder against
//! libvpx, the ignored `*_against_libvpx` tests in `src/native/mod.rs`, which
//! are the measurements `docs/benchmarks.md` is made from, through the
//! crate's public API, with lines a capture or keyframe can be held to. Run
//! by hand when wanted; no step of the workflow runs it. Over the wlshare
//! captures in `tmp/samples/` (or the directory `CAPTURES` names; `ONLY`
//! keeps the captures whose name holds it), each encoder through a libvpx
//! decoder of its own, a line printed as the tests print it, so that
//! `tools/tables.py` reads it too. Without `SPEED=fastest`, since `Speed` is
//! not public. Run in release on an idle machine, one at a time:
//!
//! ```text
//! cargo run --release --features native --example against_libvpx -- captures
//! cargo run --release --features native --example against_libvpx -- keyframes
//! cargo run --release --features native --example against_libvpx -- settles
//! ```
//!
//! `captures` and `keyframes` exit 1 where a capture over all its frames or a
//! keyframe at a dial is past the gate's lines (`GATE_DB`, `GATE_BYTES`,
//! `GATE_TIME`), each named; `settles` is not held to them. `THREADS` (4 if
//! unset) is the encoders' threads; `ALONE` leaves libvpx out, for a profiler
//! to watch the Rust coder or for one build of it to be set against another,
//! and the gate with it.

use screen_vp9::{Coder, Decoded, Decoder, Encoder, Error, Picture, Rect};

/// The lines `captures` and `keyframes` hold the Rust coder to, a capture
/// over all its frames and a keyframe at each dial: no more than a tenth of a
/// decibel under libvpx's luma PSNR, no more than 1.10 of its bytes and 1.15
/// of its time, which is wide of the tenth a time on this machine moves by
/// from run to run. The settles are not held to them: a settle from 65 is up
/// to 1.6 of libvpx's time, and one from 89 up to 1.55 of its bytes.
const GATE_DB: f64 = -0.1;
const GATE_BYTES: f64 = 1.10;
const GATE_TIME: f64 = 1.15;

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let past = match which.as_str() {
        "captures" => captures_against_libvpx(),
        "keyframes" => keyframes_against_libvpx(),
        "settles" => {
            settles_against_libvpx();
            Vec::new()
        }
        _ => {
            eprintln!("usage: against_libvpx captures|keyframes|settles");
            std::process::exit(2);
        }
    };
    if !past.is_empty() {
        eprintln!("past the gate's lines:\n{}", past.join("\n"));
        std::process::exit(1);
    }
}

/// What of the Rust coder's bytes and time over libvpx's and its luma PSNR
/// less libvpx's is past the gate's lines, if any.
fn past_the_gate(bytes: f64, time: f64, db: f64) -> Option<String> {
    let past: Vec<String> = [
        (db < GATE_DB).then(|| format!("{db:+.2} dB")),
        (bytes > GATE_BYTES).then(|| format!("bytes {bytes:.3}")),
        (time > GATE_TIME).then(|| format!("time {time:.3}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!past.is_empty()).then(|| past.join(", "))
}

/// Whether the capture at `path` is held to the gate's lines in this run:
/// libvpx measured beside it, and not `flood`, which is context for a
/// measurement and gates nothing.
fn gated(path: &std::path::Path) -> bool {
    !alone() && !capture_name(path).starts_with("flood-")
}

/// Whether `ALONE` is set: the measurements then leave libvpx out.
fn alone() -> bool {
    std::env::var_os("ALONE").is_some()
}

fn psnr(mse: f64) -> f64 {
    10.0 * (255.0 * 255.0 / mse).log10()
}

/// The mean squared error between the planes of `picture` and those of a
/// decode, over the picture.
fn mse(picture: &Picture, decoded: &Decoded<'_>) -> f64 {
    let (w, h) = picture.size();
    let (w, h) = (usize::from(w), usize::from(h));
    let mut sum = 0.0;
    for ((source, plane), stride) in picture.planes().into_iter().zip(decoded.planes()).zip(decoded.strides()) {
        for y in 0..h {
            for x in 0..w {
                sum += (f64::from(source[y * w + x]) - f64::from(plane[y * stride + x])).powi(2);
            }
        }
    }
    sum / (3 * w * h) as f64
}

/// The mean squared error of a decode's luma against `picture`'s.
fn luma_mse(picture: &Picture, decoded: &Decoded<'_>) -> f64 {
    let (w, h) = picture.size();
    let (w, h) = (usize::from(w), usize::from(h));
    let (source, plane, stride) = (picture.planes()[0], decoded.planes()[0], decoded.strides()[0]);
    let mut sum = 0.0;
    for y in 0..h {
        for (a, b) in source[y * w..][..w].iter().zip(&plane[y * stride..][..w]) {
            sum += (f64::from(*a) - f64::from(*b)).powi(2);
        }
    }
    sum / (w * h) as f64
}

/// One frame of a wlshare capture, what a session handed its stream: the
/// file `crates/wlshare-rfb/src/capture.rs` writes there, read here from its
/// documented layout.
struct Captured<'a> {
    size: (u16, u16),
    quality: u8,
    keyframe: bool,
    changed: Option<&'a [Rect]>,
    /// The whole picture as it stands with this frame, `B, G, R, X`, rows
    /// tight at the width.
    pixels: &'a [u8],
}

/// The rows `rects` span of a picture `height` high, each once, merged and
/// in order: the rows a frame of rectangles carries.
fn spans(rects: &[Rect], height: u16) -> Vec<std::ops::Range<u16>> {
    let mut spans: Vec<std::ops::Range<u16>> = rects.iter().map(|r| r.y.min(height)..r.y.saturating_add(r.height).min(height)).filter(|s| !s.is_empty()).collect();
    spans.sort_by_key(|s| s.start);
    let mut merged: Vec<std::ops::Range<u16>> = Vec::new();
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

/// Hand `f` each frame of the capture at `path`: a `.wlcap`, or one `zstd`
/// compressed as `.wlcap.zst`, read through `zstd -dc`.
fn each_captured(path: &std::path::Path, mut f: impl FnMut(&Captured<'_>) -> bool) {
    use std::io::Read as _;
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut zstd = None;
    let mut input: Box<dyn std::io::Read> = if path.extension().is_some_and(|e| e == "zst") {
        // Its complaint when the read stops short is nothing to print.
        let mut child = std::process::Command::new("zstd").args(["-dc", "-q"]).stdin(file).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().expect("zstd on the path");
        let out = child.stdout.take().expect("piped");
        zstd = Some(child);
        Box::new(std::io::BufReader::with_capacity(1 << 20, out))
    } else {
        Box::new(std::io::BufReader::with_capacity(1 << 20, file))
    };
    let mut magic = [0; 8];
    input.read_exact(&mut magic).expect("a capture's magic");
    assert_eq!(&magic, b"WLSHCAP1", "{}: not a wlshare capture", path.display());
    let (mut pixels, mut rects, mut size) = (Vec::new(), Vec::new(), (0u16, 0u16));
    let mut header = [0u8; 16];
    loop {
        match input.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("{}: {e}", path.display()),
        }
        let u16_at = |at: usize| u16::from_le_bytes([header[at], header[at + 1]]);
        let (w, h, quality, flags, count) = (u16_at(8), u16_at(10), header[12], header[13], u16_at(14));
        rects.clear();
        for _ in 0..count {
            let mut r = [0; 8];
            input.read_exact(&mut r).expect("a rectangle");
            let field = |at: usize| u16::from_le_bytes([r[at], r[at + 1]]);
            rects.push(Rect { x: field(0), y: field(2), width: field(4), height: field(6) });
        }
        let row = usize::from(w) * 4;
        let spans = if flags & 2 == 0 {
            if size != (w, h) {
                size = (w, h);
                pixels.clear();
                pixels.resize(row * usize::from(h), 0);
            }
            std::iter::once(0..h).collect()
        } else {
            assert_eq!(size, (w, h), "rectangles of a picture no frame before holds");
            spans(&rects, h)
        };
        for span in spans {
            input.read_exact(&mut pixels[usize::from(span.start) * row..usize::from(span.end) * row]).expect("a whole frame");
        }
        if !f(&Captured { size, quality, keyframe: flags & 1 != 0, changed: (flags & 2 != 0).then_some(&rects[..]), pixels: &pixels }) {
            break;
        }
    }
    drop(input);
    if let Some(mut child) = zstd {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// What a captured frame was, the kinds measured apart.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// A keyframe: asked, or the first at a size.
    Key,
    /// The whole picture as an inter frame: a settle, or a frame its session
    /// knew no damage for.
    Whole,
    /// Rectangles within a superblock's area.
    Small,
    /// Rectangles within a sixteenth of the picture.
    Mid,
    /// Rectangles over more.
    Large,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Key => "key",
            Kind::Whole => "whole",
            Kind::Small => "small",
            Kind::Mid => "mid",
            Kind::Large => "large",
        }
    }
}

/// Both encoders' bytes, times and luma PSNRs over the frames of one kind,
/// libvpx's first, and the ratio of the two times frame by frame.
#[derive(Default)]
struct Tally {
    frames: usize,
    bytes: [usize; 2],
    times: [Vec<f64>; 2],
    psnr: [f64; 2],
    ratios: Vec<f64>,
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_unstable_by(f64::total_cmp);
    v[v.len() / 2]
}

/// `frames`, each a VP9 frame of a `width`×`height` stream, as an IVF file
/// at `path`, for another decoder to read.
fn write_ivf(path: &std::path::Path, (width, height): (u16, u16), frames: &[Vec<u8>]) {
    let mut ivf = Vec::new();
    ivf.extend(b"DKIF");
    ivf.extend(0u16.to_le_bytes());
    ivf.extend(32u16.to_le_bytes());
    ivf.extend(b"VP90");
    ivf.extend(width.to_le_bytes());
    ivf.extend(height.to_le_bytes());
    ivf.extend(30u32.to_le_bytes());
    ivf.extend(1u32.to_le_bytes());
    ivf.extend((frames.len() as u32).to_le_bytes());
    ivf.extend(0u32.to_le_bytes());
    for (i, frame) in frames.iter().enumerate() {
        ivf.extend((frame.len() as u32).to_le_bytes());
        ivf.extend((i as u64).to_le_bytes());
        ivf.extend(frame);
    }
    std::fs::write(path, ivf).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// Play the capture at `path` into the Rust coder and into libvpx as the
/// session played it into its stream: the same rectangles, the same dial, a
/// keyframe where one was asked, on `threads`, which of the two goes first
/// alternating frame by frame so that the machine's load lands on both,
/// every frame of each through a libvpx decoder of its own. Per kind of
/// frame, what each made of it.
fn play(path: &std::path::Path, threads: usize) -> (std::collections::BTreeMap<Kind, Tally>, (u16, u16), usize) {
    let against = !alone();
    let mut stream: Option<(Encoder, Encoder, [Decoder; 2], Picture)> = None;
    let mut tallies: std::collections::BTreeMap<Kind, Tally> = std::collections::BTreeMap::new();
    let (mut n, mut size, mut libvpx_carried) = (0usize, (0u16, 0u16), false);
    // With `IVF` set to a directory, both streams are kept and written there
    // as `<capture>-<WxH>-<encoder>.ivf`, a file per size.
    let ivf = std::env::var_os("IVF").map(std::path::PathBuf::from);
    let frames = std::env::var_os("FRAMES").is_some();
    type Kept = ((u16, u16), [Vec<Vec<u8>>; 2]);
    let mut kept: Vec<Kept> = Vec::new();
    each_captured(path, |frame| {
        let (w, h) = frame.size;
        let fresh = size != (w, h);
        if fresh {
            size = (w, h);
            let libvpx = Encoder::new(w, h, frame.quality, threads, Coder::Libvpx).expect("libvpx");
            let rust = Encoder::new(w, h, frame.quality, threads, Coder::Native).expect("an encoder");
            stream = Some((libvpx, rust, [Decoder::new(1).expect("a decoder"), Decoder::new(1).expect("a decoder")], Picture::new(w, h, threads).expect("a picture")));
            libvpx_carried = false;
        }
        let (libvpx, rust, decoders, picture) = stream.as_mut().expect("made above");
        let stride = usize::from(w) * 4;
        match frame.changed {
            Some(rects) => {
                for r in rects {
                    picture.read_bgrx_rect(frame.pixels, stride, *r).expect("a rectangle of the picture");
                }
            }
            None => picture.read_bgrx(frame.pixels, stride).expect("the picture"),
        }
        if libvpx.quality() != frame.quality {
            libvpx.set_quality(frame.quality);
        }
        if rust.quality() != frame.quality {
            rust.set_quality(frame.quality);
        }
        let kind = match frame.changed {
            _ if frame.keyframe || fresh => Kind::Key,
            None => Kind::Whole,
            Some(rects) => {
                let area: usize = rects.iter().map(|r| usize::from(r.width.min(w.saturating_sub(r.x))) * usize::from(r.height.min(h.saturating_sub(r.y)))).sum();
                if area <= 64 * 64 {
                    Kind::Small
                } else if area * 16 <= usize::from(w) * usize::from(h) {
                    Kind::Mid
                } else {
                    Kind::Large
                }
            }
        };
        let mut out = [Vec::new(), Vec::new()];
        let mut times = [0.0; 2];
        let mut produced = true;
        for c in if n % 2 == 0 { [0, 1] } else { [1, 0] } {
            if c == 0 && !against {
                continue;
            }
            let started = std::time::Instant::now();
            if c == 0 {
                // libvpx is handed the rectangles as its stream would be:
                // not after a frame it produced nothing for.
                produced = match libvpx.encode(picture, frame.keyframe, frame.changed.filter(|_| libvpx_carried), &mut out[0]) {
                    Ok(_) => true,
                    Err(Error::NoFrame) => false,
                    Err(e) => panic!("libvpx: {e}"),
                };
            } else {
                rust.encode(picture, frame.keyframe, frame.changed, &mut out[1]).expect("a frame");
            }
            times[c] = started.elapsed().as_secs_f64() * 1000.0;
        }
        libvpx_carried = produced;
        let tally = tallies.entry(kind).or_default();
        tally.frames += 1;
        let mut quality = [0.0; 2];
        for c in 0..2 {
            if c == 0 && (!against || !produced) {
                continue;
            }
            let decoded = decoders[c].decode(&out[c]).expect("a frame libvpx decodes");
            tally.bytes[c] += out[c].len();
            tally.times[c].push(times[c]);
            quality[c] = psnr(luma_mse(picture, &decoded).max(1e-3));
            tally.psnr[c] += quality[c];
        }
        if against && produced {
            tally.ratios.push(times[1] / times[0]);
        }
        // With `FRAMES` set, each frame on a line of its own: its number,
        // dial and kind, and both encoders' bytes, time and PSNR.
        if frames {
            println!(
                "    #{n:<4} q{:<3} {:<5} | libvpx {:>8} B {:>6.2} ms {:>5.2} dB | rust {:>8} B {:>6.2} ms {:>5.2} dB",
                frame.quality,
                kind.name(),
                out[0].len(),
                times[0],
                quality[0],
                out[1].len(),
                times[1],
                quality[1]
            );
        }
        if ivf.is_some() {
            if fresh {
                kept.push(((w, h), [Vec::new(), Vec::new()]));
            }
            let streams = &mut kept.last_mut().expect("pushed above").1;
            if against && produced {
                streams[0].push(std::mem::take(&mut out[0]));
            }
            streams[1].push(std::mem::take(&mut out[1]));
        }
        n += 1;
        true
    });
    if let Some(dir) = ivf {
        for ((w, h), streams) in &kept {
            for (stream, coder) in streams.iter().zip(["libvpx", "rust"]).filter(|(stream, _)| !stream.is_empty()) {
                write_ivf(&dir.join(format!("{}-{w}x{h}-{coder}.ivf", capture_name(path))), (*w, *h), stream);
            }
        }
    }
    (tallies, size, n)
}

/// The captures in the directory `CAPTURES` names (`tmp/samples` if unset,
/// the cut captures the tables are measured on), in name order.
fn captures() -> Vec<std::path::PathBuf> {
    let dir = std::env::var_os("CAPTURES").map_or_else(|| std::path::PathBuf::from("tmp/samples"), std::path::PathBuf::from);
    let only = std::env::var("ONLY").ok();
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("an entry").path())
        .filter(|p| p.to_str().is_some_and(|s| s.ends_with(".wlcap") || s.ends_with(".wlcap.zst")))
        .filter(|p| only.as_ref().is_none_or(|only| p.to_str().is_some_and(|s| s.contains(only.as_str()))))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no captures in {}", dir.display());
    paths
}

fn capture_name(path: &std::path::Path) -> String {
    path.file_name().expect("a file").to_string_lossy().trim_end_matches(".zst").trim_end_matches(".wlcap").to_owned()
}

/// The captures whose first picture is a video paused on its first frame,
/// which is black, and says nothing coded alone; `walk` ends on the video
/// paused where it stood, the picture its session settles.
const PAUSED: [&str; 2] = ["video", "walk"];

/// The picture a keyframe and a settle of a capture are measured on: what
/// its first frame holds, or for a capture that starts on a paused video
/// (`PAUSED`), what its last frame of more than one colour does, the video
/// as it last stood.
fn measured_picture(path: &std::path::Path) -> Picture {
    let name = capture_name(path);
    let last = PAUSED.iter().any(|paused| name.starts_with(&format!("{paused}-")));
    let (mut size, mut pixels) = ((0, 0), Vec::new());
    each_captured(path, |frame| {
        // `B, G, R, X`: the fourth byte is not the picture's.
        let one_colour = frame.pixels.as_chunks::<4>().0.iter().all(|p| p[..3] == frame.pixels[..3]);
        if pixels.is_empty() || !one_colour {
            size = frame.size;
            pixels.clear();
            pixels.extend_from_slice(frame.pixels);
        }
        last
    });
    let (w, h) = size;
    assert!(!pixels.is_empty(), "{}: no frame", path.display());
    let mut picture = Picture::new(w, h, 1).expect("a picture");
    picture.read_bgrx(&pixels, usize::from(w) * 4).expect("the picture");
    picture
}

/// One frame's bytes, least time and PSNR, luma's and over all three
/// planes, from each encoder, libvpx's first, on a line.
fn pair_line(label: &str, r: &[(usize, f64, f64, f64); 2]) {
    println!(
        "  {label:<8} | libvpx {:>8} B {:>6.2} ms {:>5.2} dB | rust {:>8} B {:>6.2} ms {:>5.2} dB | bytes {:.3} time {:.3} dB {:+.2} all {:+.2}",
        r[0].0, r[0].1, r[0].2, r[1].0, r[1].1, r[1].2,
        r[1].0 as f64 / r[0].0.max(1) as f64, r[1].1 / r[0].1, r[1].2 - r[0].2, r[1].3 - r[0].3,
    );
}

fn threads() -> usize {
    std::env::var("THREADS").map_or(4, |t| t.parse().expect("a thread count"))
}

fn passes(default: usize) -> usize {
    std::env::var("PASSES").map_or(default, |t| t.parse().expect("a pass count"))
}

/// The picture of each capture `measured_picture` takes, its first or a
/// paused video's last, coded as a keyframe by both encoders at the dials
/// `DIALS` names (`40,65,90` if unset), each `PASSES` times (5 if unset)
/// turn about for the least time: bytes, time and PSNR per dial, luma's and
/// over all three planes. With `IVF` set to a directory, each keyframe is
/// written there as `<capture>-q<dial>-<encoder>.ivf`. What is past the
/// gate's lines, as `gated` holds a capture to them.
fn keyframes_against_libvpx() -> Vec<String> {
    let threads = threads();
    let dials: Vec<u8> = std::env::var("DIALS").map_or_else(|_| vec![40, 65, 90], |d| d.split(',').map(|v| v.parse().expect("a dial")).collect());
    let passes = passes(5);
    let mut past = Vec::new();
    for path in captures() {
        let picture = measured_picture(&path);
        let (w, h) = picture.size();
        println!("{} {w}x{h} on {threads}", capture_name(&path));
        for &dial in &dials {
            let mut libvpx = Encoder::new(w, h, dial, threads, Coder::Libvpx).expect("libvpx");
            let mut rust = Encoder::new(w, h, dial, threads, Coder::Native).expect("an encoder");
            let mut result = [(0usize, f64::INFINITY, 0.0f64, 0.0f64); 2];
            let mut frames = [Vec::new(), Vec::new()];
            for _ in 0..passes {
                for c in 0..2 {
                    if c == 0 && alone() {
                        continue;
                    }
                    frames[c].clear();
                    let started = std::time::Instant::now();
                    if c == 0 {
                        libvpx.encode(&picture, true, None, &mut frames[0]).expect("a frame");
                    } else {
                        rust.encode(&picture, true, None, &mut frames[1]).expect("a keyframe");
                    }
                    result[c].1 = result[c].1.min(started.elapsed().as_secs_f64() * 1000.0);
                }
            }
            for c in 0..2 {
                if c == 0 && alone() {
                    continue;
                }
                let mut decoder = Decoder::new(1).expect("a decoder");
                let decoded = decoder.decode(&frames[c]).expect("a frame libvpx decodes");
                result[c].0 = frames[c].len();
                result[c].2 = psnr(luma_mse(&picture, &decoded).max(1e-3));
                result[c].3 = psnr(mse(&picture, &decoded).max(1e-3));
                if let Some(dir) = std::env::var_os("IVF") {
                    write_ivf(&std::path::Path::new(&dir).join(format!("{}-q{dial}-{}.ivf", capture_name(&path), ["libvpx", "rust"][c])), (w, h), std::slice::from_ref(&frames[c]));
                }
            }
            pair_line(&format!("q{dial}"), &result);
            if let Some(what) = gated(&path).then(|| past_the_gate(result[1].0 as f64 / result[0].0.max(1) as f64, result[1].1 / result[0].1, result[1].2 - result[0].2)).flatten() {
                println!("  past the gate: {what}");
                past.push(format!("{} q{dial}: {what}", capture_name(&path)));
            }
        }
    }
    past
}

/// The picture of each capture `measured_picture` takes, its first or a
/// paused video's last, coded as a keyframe at the first dial of each pair
/// `SETTLES` names (`20:90,65:90,80:90,89:90` if unset), and then settled at
/// the second, the picture unchanged, as a caller sharpens a quiet picture
/// after the dial walked: the Rust coder's `settle`, and libvpx's whole
/// frame at the second dial, which is what its settle is. Each is coded
/// `PASSES` times (5 if unset) from the same keyframe, turn about, for the
/// least time; its bytes, time and PSNR per pair. With `IVF` set to a
/// directory, each keyframe and its settle are written there as
/// `<capture>-q<from>-<to>-<encoder>.ivf`.
fn settles_against_libvpx() {
    let threads = threads();
    let pairs: Vec<(u8, u8)> = std::env::var("SETTLES").map_or_else(
        |_| vec![(20, 90), (65, 90), (80, 90), (89, 90)],
        |d| d.split(',').map(|p| p.split_once(':').map(|(a, b)| (a.parse().expect("a dial"), b.parse().expect("a dial"))).expect("from:to")).collect(),
    );
    let passes = passes(5);
    for path in captures() {
        let picture = measured_picture(&path);
        let (w, h) = picture.size();
        println!("{} {w}x{h} on {threads}", capture_name(&path));
        for &(from, to) in &pairs {
            let mut libvpx = Encoder::new(w, h, from, threads, Coder::Libvpx).expect("libvpx");
            let mut rust = Encoder::new(w, h, from, threads, Coder::Native).expect("an encoder");
            let mut result = [(0usize, f64::INFINITY, 0.0f64, 0.0f64); 2];
            for _ in 0..passes {
                for (c, r) in result.iter_mut().enumerate() {
                    if c == 0 && alone() {
                        continue;
                    }
                    let mut frames = [Vec::new(), Vec::new()];
                    let time = if c == 0 {
                        libvpx.set_quality(from);
                        libvpx.encode(&picture, true, None, &mut frames[0]).expect("a frame");
                        libvpx.set_quality(to);
                        let started = std::time::Instant::now();
                        libvpx.encode(&picture, false, None, &mut frames[1]).expect("a frame");
                        started.elapsed()
                    } else {
                        rust.encode(&picture, true, None, &mut frames[0]).expect("a keyframe");
                        let started = std::time::Instant::now();
                        rust.settle(&picture, to, false, &mut frames[1]).expect("a settle");
                        started.elapsed()
                    };
                    r.1 = r.1.min(time.as_secs_f64() * 1000.0);
                    let mut decoder = Decoder::new(1).expect("a decoder");
                    decoder.decode(&frames[0]).expect("a keyframe libvpx decodes");
                    let decoded = decoder.decode(&frames[1]).expect("a settle libvpx decodes");
                    r.0 = frames[1].len();
                    r.2 = psnr(luma_mse(&picture, &decoded).max(1e-3));
                    r.3 = psnr(mse(&picture, &decoded).max(1e-3));
                    if let Some(dir) = std::env::var_os("IVF") {
                        write_ivf(&std::path::Path::new(&dir).join(format!("{}-q{from}-{to}-{}.ivf", capture_name(&path), ["libvpx", "rust"][c])), (w, h), &frames);
                    }
                }
            }
            pair_line(&format!("q{from}:{to}"), &result);
        }
    }
}

/// Every frame of each capture, played into the Rust coder and into libvpx
/// as the session played it into its stream, on `THREADS` threads, `PASSES`
/// times over (1 if unset). Per capture, for the frames of each kind —
/// keyframes, whole inter frames, and rectangles within a superblock, within
/// a sixteenth of the picture and over more — each encoder's bytes, median
/// time and the luma PSNR of libvpx's decode, and the ratios
/// `tools/tables.py` makes the tables of `docs/benchmarks.md` from. `FRAMES`
/// prints every frame. With `IVF` set to a directory, both encoders'
/// streams of each capture are written there too, for the browsers' own
/// decoders to read through ffmpeg (`-c:v libvpx-vp9` is Chrome's, `-c:v
/// vp9` Firefox's; `-f framemd5` of each should agree), one file per size
/// the capture runs at, since ffmpeg rescales every frame after a size
/// change to the first frame's size before hashing it. What is past the
/// gate's lines over all of a capture's frames, the time the median of the
/// per-frame ratios, as `gated` holds a capture to them.
fn captures_against_libvpx() -> Vec<String> {
    let threads = threads();
    let passes = passes(1);
    let mut past = Vec::new();
    for path in captures() {
        for _ in 0..passes {
            let (mut tallies, size, n) = play(&path, threads);
            println!("{} {}x{} on {threads}, {n} frames", capture_name(&path), size.0, size.1);
            let mut all = Tally::default();
            // Its bytes over libvpx's, the median time ratio and the PSNR
            // difference, for the gate.
            let line = |name: &str, tally: &mut Tally| {
                let made = [tally.times[0].len().max(1) as f64, tally.frames as f64];
                let bytes = tally.bytes[1] as f64 / tally.bytes[0].max(1) as f64;
                let time = median(&mut tally.ratios);
                let db = tally.psnr[1] / made[1] - tally.psnr[0] / made[0];
                println!(
                    "  {name:<5} {:>5} | libvpx {:>9} B {:>6.2} ms {:>5.2} dB | rust {:>9} B {:>6.2} ms {:>5.2} dB | bytes {:.3} time median {:.3} total {:.3}",
                    tally.frames,
                    tally.bytes[0],
                    median(&mut tally.times[0]),
                    tally.psnr[0] / made[0],
                    tally.bytes[1],
                    median(&mut tally.times[1]),
                    tally.psnr[1] / made[1],
                    bytes,
                    time,
                    tally.times[1].iter().sum::<f64>() / tally.times[0].iter().sum::<f64>().max(1e-9),
                );
                (bytes, time, db)
            };
            for (kind, tally) in &mut tallies {
                line(kind.name(), tally);
                all.frames += tally.frames;
                for c in 0..2 {
                    all.bytes[c] += tally.bytes[c];
                    all.times[c].extend_from_slice(&tally.times[c]);
                    all.psnr[c] += tally.psnr[c];
                }
                all.ratios.extend_from_slice(&tally.ratios);
            }
            let (bytes, time, db) = line("all", &mut all);
            if let Some(what) = gated(&path).then(|| past_the_gate(bytes, time, db)).flatten() {
                println!("  past the gate: {what}");
                past.push(format!("{}: {what}", capture_name(&path)));
            }
        }
    }
    past
}
