//! Two encoders timed against each other on a captured stream, interleaved.
//!
//! An IVF capture is decoded to the `B, G, R, X` a framebuffer holds, converted
//! once to a [`Picture`], and that one picture is handed to both candidates back
//! to back. Which goes first flips every frame and again every pass, so neither
//! is always the one on the warmer core, and what is reported is the ratio of the
//! two times *for the same frame*: a burst of load on the machine lands on both
//! halves of a pair rather than on one candidate's whole run. Each frame's time
//! is the least it took over the passes.
//!
//! Beside the times are the bytes each made and the PSNR of what libvpx's
//! decoder — independent of either candidate — makes of them against the planes
//! that went in, so a candidate that is quick by coding less shows it.
//!
//! Naming the same candidate twice measures the harness: the ratio should be 1,
//! and how far its spread strays is the smallest difference a run can show.
//!
//! ```text
//! cargo run --release --example bench -- capture.ivf QUALITY THREADS [--passes N] [--a NAME] [--b NAME] [--csv PATH]
//! ```

use std::io::Write;
use std::time::Instant;

use screen_vp9::{Chroma, Decoder, Encoder, Picture, Speed};

/// One candidate: a picture in, a frame out, and whether it was a keyframe.
type Encode = Box<dyn FnMut(&Picture, &mut Vec<u8>) -> bool>;

/// The candidates a run can name.
const CANDIDATES: [&str; 2] = ["libvpx", "libvpx-fastest"];

/// The candidate `name` is, for a stream of this shape.
fn candidate(name: &str, size: (u16, u16), chroma: Chroma, quality: u8, threads: usize) -> Encode {
    let speed = match name {
        "libvpx" => Speed::Usual,
        "libvpx-fastest" => Speed::Fastest,
        other => unreachable!("{other} was checked against CANDIDATES"),
    };
    let mut encoder = Encoder::new(size.0, size.1, chroma, quality, threads).expect("a libvpx encoder");
    encoder.set_speed(speed).expect("a speed libvpx takes");
    Box::new(move |picture, out| encoder.encode(picture, false, out).expect("an encode").expect("a frame for every picture"))
}

/// What a run was asked for.
struct Args {
    capture: String,
    quality: u8,
    threads: usize,
    passes: usize,
    names: [String; 2],
    csv: Option<String>,
}

fn usage() -> ! {
    eprintln!("usage: bench capture.ivf QUALITY THREADS [--passes N] [--a NAME] [--b NAME] [--csv PATH]");
    eprintln!("candidates: {}", CANDIDATES.join(", "));
    std::process::exit(2)
}

fn args() -> Args {
    let mut given = std::env::args().skip(1);
    let mut positional = Vec::new();
    let mut args = Args { capture: String::new(), quality: 0, threads: 0, passes: 3, names: ["libvpx".into(), "libvpx".into()], csv: None };
    while let Some(arg) = given.next() {
        let mut value = || given.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--passes" => args.passes = value().parse().unwrap_or_else(|_| usage()),
            "--a" => args.names[0] = value(),
            "--b" => args.names[1] = value(),
            "--csv" => args.csv = Some(value()),
            _ => positional.push(arg),
        }
    }
    let [capture, quality, threads] = <[String; 3]>::try_from(positional).unwrap_or_else(|_| usage());
    args.capture = capture;
    args.quality = quality.parse().unwrap_or_else(|_| usage());
    args.threads = threads.parse().unwrap_or_else(|_| usage());
    if args.passes == 0 || args.names.iter().any(|name| !CANDIDATES.contains(&name.as_str())) {
        usage();
    }
    args
}

/// The frames of an IVF file, in order.
fn ivf_frames(ivf: &[u8]) -> Vec<&[u8]> {
    assert!(ivf.len() >= 32 && &ivf[..4] == b"DKIF", "not an IVF file");
    let mut frames = Vec::new();
    let mut at = 32;
    while at + 12 <= ivf.len() {
        let len = u32::from_le_bytes(ivf[at..at + 4].try_into().expect("four bytes")) as usize;
        frames.push(&ivf[at + 12..at + 12 + len]);
        at += 12 + len;
    }
    frames
}

/// The squared error of a `width`×`height` plane against another, each at its own stride.
fn sse(got: &[u8], got_stride: usize, want: &[u8], want_stride: usize, width: usize, height: usize) -> u64 {
    (0..height)
        .map(|y| {
            let (got, want) = (&got[y * got_stride..y * got_stride + width], &want[y * want_stride..y * want_stride + width]);
            got.iter().zip(want).map(|(g, w)| u64::from(g.abs_diff(*w)).pow(2)).sum::<u64>()
        })
        .sum()
}

fn psnr(sse: u64, samples: usize) -> f64 {
    if sse == 0 { 99.0 } else { 10.0 * (255.0f64 * 255.0 * samples as f64 / sse as f64).log10() }
}

/// What a decoder makes of `frame` against the planes of `picture`: the PSNR of
/// the luma, and of the two chroma planes together.
fn quality_of(decoder: &mut Decoder, frame: &[u8], picture: &Picture) -> (f64, f64) {
    let shown = decoder.decode(frame).expect("a frame libvpx decodes");
    let (w, h) = (usize::from(picture.size().0), usize::from(picture.size().1));
    let (cw, ch) = match picture.chroma() {
        Chroma::Subsampled => (w.div_ceil(2), h.div_ceil(2)),
        Chroma::Full => (w, h),
    };
    let plane = |i: usize, w: usize, h: usize| sse(shown.planes()[i], shown.strides()[i], picture.planes()[i], picture.strides()[i], w, h);
    (psnr(plane(0, w, h), w * h), psnr(plane(1, cw, ch) + plane(2, cw, ch), 2 * cw * ch))
}

/// The value `p` of the way through `values`, which are sorted.
fn at(values: &[f64], p: f64) -> f64 {
    values[((values.len() - 1) as f64 * p).round() as usize]
}

fn sorted(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut values: Vec<f64> = values.collect();
    values.sort_unstable_by(f64::total_cmp);
    values
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn main() {
    let args = args();
    let ivf = std::fs::read(&args.capture).unwrap_or_else(|e| panic!("reading {}: {e}", args.capture));
    let frames = ivf_frames(&ivf);
    assert!(!frames.is_empty(), "an empty capture");

    // Per candidate, per frame: the least time over the passes, in milliseconds,
    // and from the first pass the bytes and what a decoder made of them.
    let mut times = [vec![f64::INFINITY; frames.len()], vec![f64::INFINITY; frames.len()]];
    let mut bytes = [vec![0usize; frames.len()], vec![0usize; frames.len()]];
    let mut quality = [vec![(0.0, 0.0); frames.len()], vec![(0.0, 0.0); frames.len()]];
    let mut keyframes = [vec![false; frames.len()], vec![false; frames.len()]];
    let mut convert = vec![f64::INFINITY; frames.len()];
    let mut shape = None;

    for pass in 0..args.passes {
        // Every pass starts each candidate over, as the capture's stream started.
        let mut source = Decoder::new(4).expect("a decoder");
        let mut checks = [Decoder::new(4).expect("a decoder"), Decoder::new(4).expect("a decoder")];
        let mut outs = [Vec::new(), Vec::new()];
        let mut run: Option<(Picture, Vec<u8>, [Encode; 2])> = None;

        for (n, frame) in frames.iter().enumerate() {
            // Untimed: the raw frame, as the B, G, R, X a capture hands a user.
            let decoded = source.decode(frame).expect("a capture libvpx decodes");
            let (size, chroma) = (decoded.size(), decoded.chroma());
            assert!(*shape.get_or_insert((size, chroma)) == (size, chroma), "the capture changes size or chroma mid-stream");
            let (picture, bgrx, encoders) = run.get_or_insert_with(|| {
                let coded = (u16::try_from(size.0).expect("a VP9 width"), u16::try_from(size.1).expect("a VP9 height"));
                let encoders = [0, 1].map(|c| candidate(&args.names[c], coded, chroma, args.quality, args.threads));
                (Picture::new(coded.0, coded.1, chroma).expect("a picture"), vec![0u8; size.0 as usize * size.1 as usize * 4], encoders)
            });
            let stride = size.0 as usize * 4;
            decoded.write_bgrx(bgrx, stride).expect("a buffer of the frame's size");

            // The conversion is the crate's and the same in front of either candidate.
            let started = Instant::now();
            picture.read_bgrx(bgrx, stride).expect("a buffer of the picture's size");
            convert[n] = convert[n].min(started.elapsed().as_secs_f64() * 1000.0);

            let order = if (n + pass).is_multiple_of(2) { [0, 1] } else { [1, 0] };
            for c in order {
                outs[c].clear();
                let started = Instant::now();
                let keyframe = encoders[c](picture, &mut outs[c]);
                times[c][n] = times[c][n].min(started.elapsed().as_secs_f64() * 1000.0);
                keyframes[c][n] = keyframe;
            }

            // Untimed, and once: the decoder has to follow each stream from its start.
            if pass == 0 {
                for c in 0..2 {
                    bytes[c][n] = outs[c].len();
                    quality[c][n] = quality_of(&mut checks[c], &outs[c], picture);
                }
            }
        }
    }

    let ((width, height), chroma) = shape.expect("a frame");
    println!(
        "{} | {width}x{height} {} q{} t{} | {} frames, {} passes | convert ms mean {:.2}",
        args.capture, chroma.name(), args.quality, args.threads, frames.len(), args.passes, mean(&convert),
    );
    let megabytes = |c: usize| bytes[c].iter().sum::<usize>() as f64 / 1e6;
    for (c, label) in ["a", "b"].into_iter().enumerate() {
        let ordered = sorted(times[c].iter().copied());
        println!(
            "  {label} {:<15} encode ms mean {:.2} median {:.2} p95 {:.2} max {:.2} | {:.2} MB | psnr y {:.2} uv {:.2}",
            args.names[c], mean(&ordered), at(&ordered, 0.5), at(&ordered, 0.95), at(&ordered, 1.0), megabytes(c),
            mean(&quality[c].iter().map(|q| q.0).collect::<Vec<_>>()), mean(&quality[c].iter().map(|q| q.1).collect::<Vec<_>>()),
        );
    }
    let ratios = sorted(times[0].iter().zip(&times[1]).map(|(a, b)| b / a));
    println!(
        "  b/a time: median {:.3} p5 {:.3} p95 {:.3} total {:.3} | bytes {:.3}",
        at(&ratios, 0.5), at(&ratios, 0.05), at(&ratios, 0.95), times[1].iter().sum::<f64>() / times[0].iter().sum::<f64>(), megabytes(1) / megabytes(0),
    );

    if let Some(path) = &args.csv {
        let mut csv = std::io::BufWriter::new(std::fs::File::create(path).unwrap_or_else(|e| panic!("creating {path}: {e}")));
        writeln!(csv, "frame,convert_ms,a_ms,b_ms,a_bytes,b_bytes,a_keyframe,b_keyframe,a_psnr_y,b_psnr_y,a_psnr_uv,b_psnr_uv").expect("a row written");
        for n in 0..frames.len() {
            writeln!(
                csv,
                "{n},{:.3},{:.3},{:.3},{},{},{},{},{:.2},{:.2},{:.2},{:.2}",
                convert[n], times[0][n], times[1][n], bytes[0][n], bytes[1][n], u8::from(keyframes[0][n]), u8::from(keyframes[1][n]),
                quality[0][n].0, quality[1][n].0, quality[0][n].1, quality[1][n].1,
            )
            .expect("a row written");
        }
    }
}
