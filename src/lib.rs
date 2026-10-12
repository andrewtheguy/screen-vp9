//! VP9 for a desktop picture, as wlshare and the remotex gateway both code it.
//!
//! One [`Encoder`], written for a desktop and nothing else: 8-bit 4:4:4, VP9
//! profile 1 ([`PROFILE`]), a quantizer pinned to a 1–100 quality dial rather
//! than a bitrate, no lag, no dropped frames, no keyframe that was not asked
//! for, and the colour matrix and range declared in the bitstream so a decoder
//! does not guess. It is coded by one of two coders, which its maker names
//! ([`Coder`]): libvpx set up that way, or the encoder written here in Rust,
//! which is EXPERIMENTAL and built only with the `native` feature, since its
//! build is for NEON or x86-64-v3 and one without it is for any machine
//! libvpx is. The stream is the same kind either way, and nothing
//! else a user calls differs. In front of it a [`Picture`] holds the planes it
//! reads — BT.601 at studio swing, a sample per pixel in each — converted from
//! the packed RGB the gateway's mirror holds or the `B, G, R, X` a framebuffer
//! does. Behind it a [`Decoder`], libvpx's, gives the planes back, for every
//! test here and in a user: the independent decoder every frame the Rust coder
//! makes in a test is held to.
//!
//! The stream's shape is chosen for whoever decodes it. Tile columns follow the
//! picture's width, one under 1440, two from 1440, four from 2048 and eight
//! from 5760, and never outnumber the encoder's threads. A stream in four
//! columns or more is coded without the loop filter, which is what a software
//! decoder's threads wait on there. The measurements are at `TILE_WIDTHS` and
//! `loop_filtered`.
//!
//! A frame need not cost the whole picture. [`Picture::read_bgrx_rect`] converts
//! the rectangle that changed, and [`Encoder::encode`] told where the picture changed
//! skips every block outside it, which a caller that knows its damage says and
//! one that does not leaves unsaid. [`Stream`] is the two together, and what a
//! user holds: it reads a packed picture where that changed
//! ([`Stream::read_bgrx`]), straight from wherever the pixels are, and codes
//! what it read ([`Stream::encode`]). It keeps the rules a frame of damage
//! needs — when a frame is the whole picture anyway, and how coarse the blocks
//! no frame has touched since still are — and the frame that settles them, the
//! whole picture once at the ceiling with the dial left where the link had it
//! ([`Stream::settle`]).
//!
//! Frame metadata is handled here too: [`frame_header`] reads the profile and
//! keyframe bit of a frame this process did not encode, and [`codec_string`]
//! builds the WebCodecs string a browser's `VideoDecoder` is configured with
//! from a stream's size and frame rate. VP9 carries no parameter sets a
//! client could read that string out of.
//!
//! The dial is walked here too: [`walk::QualityWalk`] gives quality up when the
//! link is behind and frame rate once the quality reaches its floor; severe lag
//! can reduce both at once. It takes them back when the link keeps up, the same
//! walk for a stream wlshare codes and one the gateway does. What is not here is
//! the rest of *when*: which picture to encode, where the link's lag is read
//! from, how a frame is framed on a wire. Each user keeps its own.

mod libvpx;
#[cfg(feature = "native")]
mod native;
pub mod walk;

pub use libvpx::{Decoded, Decoder};

use thiserror::Error;

/// The coarsest end of the quality dial.
pub const QUALITY_MIN: u8 = 1;
/// The finest end of the quality dial.
pub const QUALITY_MAX: u8 = 100;

/// The VP9 profile of every stream here: eight-bit 4:4:4, a colour sample per
/// pixel, which is what [`FrameHeader::profile`] says of one of its frames.
pub const PROFILE: u8 = 1;

/// The finest quantizer the dial reaches. VP9's range is 0–63, coarsest last;
/// below about 8 a VP9 desktop is visually lossless and costs several times the
/// bytes to be so, so a dial that mapped past it would have a top third where
/// turning the knob bought nothing but bandwidth. Settled by measurement.
const Q_FINEST: u32 = 8;
/// VP9's coarsest quantizer.
const Q_COARSEST: u32 = 63;

/// Which encoder codes a stream. Both make VP9 of the same kind from the same
/// calls, so a user that holds one holds the other by naming it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Coder {
    /// libvpx, set up for a desktop.
    #[default]
    Libvpx,
    /// The encoder written here in Rust. EXPERIMENTAL, and there only in a
    /// build with the `native` feature, which needs NEON or x86-64-v3 of
    /// the machine it is built for.
    #[cfg(feature = "native")]
    Native,
}

/// How much of its search a coder spends on a frame. Each starts at
/// [`Speed::Usual`], which is what every stream of a user's is coded at: the
/// speed is moved only by the tests and the measurements here, on a coder
/// itself. Not a leftover: the Rust coder's `Fastest` is work in progress,
/// kept for an optimization it may yet become, and not public until it is
/// one.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Speed {
    /// Every decision the Rust coder has: an intra 8×8 coded each way where
    /// that pays, and text as four 4×4s. libvpx's speed 7. For a picture that
    /// is the stream.
    #[default]
    Usual,
    /// The decisions by their proxies alone, and libvpx's speed 9: more bytes
    /// and a rougher picture for the least time in the encoder. Nothing names
    /// it in a build that is libvpx alone and no test.
    #[cfg_attr(not(any(test, feature = "native")), expect(dead_code))]
    Fastest,
}

/// The widths from which a picture is coded in two tile columns, in four and
/// in eight, so the frame header's tile columns are the log2 of that: one column under
/// 1440 pixels, two from 1440 through 1080p, four from 2048 and at 4K. The
/// width rather than the thread count decides it because a tile is a cost as
/// well as a split — the columns are coded apart, which costs bytes, and at
/// 1080p four of them coded slower than two whatever the threads — while at 4K
/// four were worth having. Two at 1440 are what a decoder's threads have to
/// split: libvpx's own decoder, which is the browser's, parses and reconstructs
/// a one-column frame on one thread and gives the others only the loop filter,
/// and vp9-wasm, which reconstructs by rows whatever the tiles, still parses a
/// column in order. A 1440×900 desktop re-encoded with two columns cost 8% more
/// bytes and vp9-wasm decoded it 10% faster on four threads; a busy Mac screen
/// of that size cost 0.1% more and decoded 21% faster. Four from 2048 and not
/// 2880 were measured on the gateway's own Mac captures at 2048×1536 and
/// 2560×1600, at quality 90 on four threads, on the 300 quietest frames of each
/// and the 300 busiest: four columns cost 0.1% more bytes than two on the quiet
/// frames and 1.4% and 1.0% on the busy, under 0.1 dB, libvpx's encoder coded
/// them 9% to 14% faster, and libvpx's four decoding threads, with a column each,
/// took 30.4 ms a busy frame for 51.9 and 30.9 for 54.3. Not from 1920: the
/// busy frames of a 1920×1080 Windows capture cost 15% more bytes in four.
/// Never more than the
/// threads can fill, since a tile no thread is free for is bytes for nothing.
const TILE_WIDTHS: [u16; 3] = [1440, 2048, 5760];

/// The most threads an encoder takes.
const MAX_THREADS: usize = 64;

/// The 1–100 quality dial as a VP9 quantizer: [`QUALITY_MIN`] is the coarsest
/// picture and becomes [`Q_COARSEST`], [`QUALITY_MAX`] the finest and becomes
/// [`Q_FINEST`]. Out-of-range input is clamped rather than refused; a
/// configuration is what rejects a bad dial.
fn quality_to_q(quality: u8) -> u32 {
    let quality = u32::from(quality.clamp(QUALITY_MIN, QUALITY_MAX));
    Q_COARSEST - (quality - 1) * (Q_COARSEST - Q_FINEST) / 99
}

/// The log2 of the tile columns a `width`-wide picture is coded in by `threads`.
fn tile_columns_log2(width: u16, threads: usize) -> u32 {
    (TILE_WIDTHS.iter().filter(|&&from| width >= from).count() as u32).min(threads.max(1).ilog2())
}

/// Whether a stream's frames are loop filtered: all but those in four tile
/// columns or more, `tile_cols_log2` being the log2 of the columns the frames
/// are in, the ones the format forces on a very wide picture among them. The filter's level is taken from the quantizer, as libvpx's
/// encoder takes it, 7 to 9 at the fine end of the dial, a light filter that
/// still visits every block edge of every frame, and it is a quarter to two
/// fifths of what vp9-wasm spends on a frame. Measured on vp9-wasm 0.0.5 with
/// what the remotex gateway wrote of one four-pane desktop at eight sizes,
/// 1280×800 to 3840×2160, each coded both ways at quality 90: without the
/// filter one thread decodes in 29% to 41% fewer cycles, and the stream is 3%
/// to 6% larger and 0.6 to 1.4 dB of luma further from its source, since every
/// later frame predicts from what the filter smoothed. What four threads gain
/// is what the tiles leave. In one or two columns a frame waits on their
/// parsing: 7.2 ms for 7.5 at 1280 wide, 5.7 for 5.8 at 1440, 6.1 for 6.4 at
/// 1600, 9.8 for 9.0 at 1920, 9.5 for 10.3 at 2560. In four: 6.4 for 10.4 at
/// 2880, 14.2 for 21.3 at 3456, 12.1 for 17.4 at 3840. An earlier 3456-wide
/// desktop coded in two columns: 27.4 for 29.0, and the 2560-wide capture in
/// four: 6.1 for 9.0, a 2048-wide one: 5.3 for 7.8. Those are each capture's
/// first 300 frames, its quietest; on its 300 busiest the filter is less of a
/// frame and of the stream, 11% to 16% of one thread's cycles for 0.2% to 1.3%
/// of the bytes and 0.3 to 0.6 dB, and four threads gain 14% to 27% in four
/// columns and nothing to speak of in one or two. So the filter goes where the
/// columns have left it as what a
/// decoder's threads wait on, which is also where a frame is slowest to decode,
/// and stays where it would be bytes and picture given for no time.
fn loop_filtered(tile_cols_log2: u32) -> bool {
    tile_cols_log2 < 2
}

/// Why a picture could not be encoded or a frame decoded.
#[derive(Debug, Error)]
pub enum Error {
    /// libvpx refused a call, in its own words.
    #[error("vp9 {call}: {detail}")]
    Codec { call: &'static str, detail: String },
    /// libvpx took the picture and made no frame of it: nothing was appended.
    /// With no lag and no dropped frames it should not happen, and it is an
    /// error and not an assertion because a caller can carry on from it, the
    /// pixels being the next frame's to carry.
    #[error("libvpx made no frame of the picture")]
    NoFrame,
    #[error("a {0}x{1} picture has no pixels")]
    Empty(u16, u16),
    #[error("an encoder for {0} threads: it takes 1 to {MAX_THREADS}")]
    Threads(usize),
    #[error("a crop of {0} bytes is not a {1}x{2} picture")]
    Crop(usize, usize, usize),
    #[error("a {width}x{height} picture at a stride of {stride} does not fit in {bytes} bytes")]
    Buffer { width: usize, height: usize, stride: usize, bytes: usize },
    #[error("a {0}x{1} encoder was handed a {2}x{3} picture")]
    Mismatch(u16, u16, u16, u16),
    #[error("a stream was asked for a frame before any picture was read into it")]
    Unread,
    #[error("the frame decoded to no picture")]
    NoPicture,
    #[error("the frame is not 8-bit 4:4:4 (format {0}, {1} bits)")]
    Format(u32, u32),
}

/// Whether `bytes` hold a `width`×`height` picture of 4-byte pixels whose rows
/// are `stride` apart, in measures the conversions' `u32` can carry. A last row
/// with nothing after it is a picture that fits.
fn fits(width: usize, height: usize, stride: usize, bytes: usize) -> bool {
    if [width, height, stride].iter().any(|&n| u32::try_from(n).is_err()) {
        return false;
    }
    width == 0 || height == 0 || (stride >= width * 4 && bytes >= (height - 1) * stride + width * 4)
}

/// A rectangle of a picture, in pixels: where it differs from the picture
/// encoded before it, for [`Encoder::encode`]. What lies past the picture's
/// edge is not part of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// One picture as the planes the encoder reads: 8-bit Y, U and V, BT.601 at
/// studio swing, a sample per pixel in each. Reused across frames, so a 1080p
/// conversion is not a 6 MB allocation apiece.
pub struct Picture {
    size: (u16, u16),
    /// Each plane with a row of nothing after its last: the conversion writes a
    /// row as a whole stride from where it starts, and a rectangle's rows start
    /// at its left edge, so its last row's stride ends up to a row past the
    /// plane.
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    /// A tight copy of the last row of a packed picture where it is not
    /// followed by a whole stride, made only for that: the conversion reads a
    /// row as a whole stride too.
    tight: Vec<u8>,
    /// How many threads a conversion is split across.
    threads: usize,
}

/// The fewest pixels a conversion gives a thread of its own, and the most
/// threads it is split across. A conversion waits on memory more than on a
/// core: a 4K picture's 33 MB of `B, G, R, X` took 5.5 ms on one thread, 3.3 on
/// two, 2.9 on three and on four and 3.0 on six, and a 1080p one 1.5, 1.0, 0.8,
/// 0.9 and 1.0. A thread also costs some 50 µs to start, which is what half a
/// megapixel's share of the gain pays for and less does not.
const THREAD_PIXELS: usize = 1 << 19;
const CONVERT_THREADS: usize = 4;

impl Picture {
    /// A buffer for a `width`×`height` picture, converted on up to `threads`
    /// threads, which are the encoder's: it is idle while its picture is
    /// read.
    pub fn new(width: u16, height: u16, threads: usize) -> Result<Self, Error> {
        if width == 0 || height == 0 {
            return Err(Error::Empty(width, height));
        }
        let (w, h) = (usize::from(width), usize::from(height));
        Ok(Self { size: (width, height), y: vec![0; w * (h + 1)], u: vec![0; w * (h + 1)], v: vec![0; w * (h + 1)], tight: Vec::new(), threads })
    }

    /// The picture's size.
    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    /// Y, U and V, each tight at the picture's width.
    pub fn planes(&self) -> [&[u8]; 3] {
        let len = usize::from(self.size.0) * usize::from(self.size.1);
        [&self.y[..len], &self.u[..len], &self.v[..len]]
    }

    /// The width of a row of each plane.
    pub fn strides(&self) -> [usize; 3] {
        [usize::from(self.size.0); 3]
    }

    /// The whole picture as a rectangle.
    fn whole(&self) -> Rect {
        Rect { x: 0, y: 0, width: self.size.0, height: self.size.1 }
    }

    /// Convert `rgb` — packed RGB888, tight, for exactly this picture — in place.
    ///
    /// The length is checked rather than trusted, because everything after the
    /// check indexes by the picture size.
    pub fn read_rgb(&mut self, rgb: &[u8]) -> Result<(), Error> {
        self.read_rgb_rect(rgb, self.whole())
    }

    /// Convert `rect` of `rgb`, which holds the whole picture as
    /// [`Self::read_rgb`] takes it, and leave every other pixel as the last
    /// conversion made it, as [`Self::read_bgrx_rect`] does.
    pub fn read_rgb_rect(&mut self, rgb: &[u8], rect: Rect) -> Result<(), Error> {
        self.holds_rgb(rgb)?;
        self.convert(rgb, usize::from(self.size.0) * 3, rect, Packed::Rgb);
        Ok(())
    }

    /// Whether `rgb` is this picture, packed and tight.
    fn holds_rgb(&self, rgb: &[u8]) -> Result<(), Error> {
        let (w, h) = (usize::from(self.size.0), usize::from(self.size.1));
        if rgb.len() != w * h * 3 {
            return Err(Error::Crop(rgb.len(), w, h));
        }
        Ok(())
    }

    /// Whether `pixels` hold this picture as `B, G, R, X`, rows `stride`
    /// bytes apart.
    fn holds_bgrx(&self, pixels: &[u8], stride: usize) -> Result<(), Error> {
        let (w, h) = (usize::from(self.size.0), usize::from(self.size.1));
        if !fits(w, h, stride, pixels.len()) {
            return Err(Error::Buffer { width: w, height: h, stride, bytes: pixels.len() });
        }
        Ok(())
    }

    /// Convert `pixels` — `B, G, R, X`, this picture's size, rows `stride` bytes
    /// apart — in place. The X byte is ignored.
    pub fn read_bgrx(&mut self, pixels: &[u8], stride: usize) -> Result<(), Error> {
        self.read_bgrx_rect(pixels, stride, self.whole())
    }

    /// Convert `rect` of `pixels`, which hold the whole picture as
    /// [`Self::read_bgrx`] takes it, and leave every other pixel as the last
    /// conversion made it: what a picture that changed in one place costs is
    /// that place and not the screen, nor the rows across it. What of `rect`
    /// lies past the picture's edge is not part of it.
    pub fn read_bgrx_rect(&mut self, pixels: &[u8], stride: usize, rect: Rect) -> Result<(), Error> {
        self.holds_bgrx(pixels, stride)?;
        self.convert(pixels, stride, rect, Packed::Bgrx);
        Ok(())
    }

    /// Convert `rect` of `pixels`, a whole picture in `packed` order that its
    /// caller has checked fits, rows `stride` bytes apart.
    fn convert(&mut self, pixels: &[u8], stride: usize, rect: Rect, packed: Packed) {
        let (w, h) = (usize::from(self.size.0), usize::from(self.size.1));
        let (left, right) = (usize::from(rect.x), (usize::from(rect.x) + usize::from(rect.width)).min(w));
        let (top, bottom) = (usize::from(rect.y), (usize::from(rect.y) + usize::from(rect.height)).min(h));
        if left >= right || top >= bottom {
            return;
        }
        // The crate reads a row as a whole stride from its first pixel, which
        // here is the rectangle's left edge. The picture's last row has only
        // what is left of its own after that, and in a buffer that ends with
        // the picture not always that much, so the row that ends the
        // rectangle is copied tight first where the buffer does not go on a
        // stride past it. The rows above have those under them to read into.
        let bytes = packed.bytes();
        let at = |row: usize| row * stride + left * bytes;
        let borrowed = if at(bottom) <= pixels.len() { bottom } else { bottom - 1 };
        self.convert_rows(&pixels[at(top)..at(borrowed)], stride, left..right, top..borrowed, packed);
        if borrowed < bottom {
            let row = (right - left) * bytes;
            let mut tight = std::mem::take(&mut self.tight);
            tight.clear();
            tight.extend_from_slice(&pixels[at(borrowed)..][..row]);
            self.convert_rows(&tight, row, left..right, borrowed..bottom, packed);
            self.tight = tight;
        }
    }

    /// Convert `cols` of `rows` from `pixels`, which begin at the first of
    /// them and give every row a whole `stride`, on as many threads as the
    /// pixels are worth.
    fn convert_rows(&mut self, pixels: &[u8], stride: usize, cols: std::ops::Range<usize>, rows: std::ops::Range<usize>, packed: Packed) {
        if rows.is_empty() {
            return;
        }
        let w = usize::from(self.size.0);
        let width = cols.len();
        let threads = self.threads.min(CONVERT_THREADS).min(width * rows.len() / THREAD_PIXELS).max(1);
        let band = rows.len().div_ceil(threads);
        let span = rows.start * w + cols.start..rows.end * w + cols.start;
        let (mut y, mut u, mut v) = (&mut self.y[span.clone()], &mut self.u[span.clone()], &mut self.v[span]);
        let mut pixels = pixels;
        std::thread::scope(|scope| {
            let mut left = rows.len();
            while left > 0 {
                let height = band.min(left);
                left -= height;
                let (band_y, band_u, band_v, band_pixels);
                (band_y, y) = y.split_at_mut(height * w);
                (band_u, u) = u.split_at_mut(height * w);
                (band_v, v) = v.split_at_mut(height * w);
                (band_pixels, pixels) = pixels.split_at(height * stride);
                let planes = [band_y, band_u, band_v];
                // The last band is this thread's own.
                if left > 0 {
                    scope.spawn(move || convert_band(band_pixels, stride, packed, planes, w, (width, height)));
                } else {
                    convert_band(band_pixels, stride, packed, planes, w, (width, height));
                }
            }
        });
    }
}

/// Convert `size` pixels from `pixels`, rows `stride` bytes apart, into the
/// `planes` Y, U and V, whose rows are `plane_stride` apart. Every row of the
/// pixels and of the planes is a whole stride.
fn convert_band(pixels: &[u8], stride: usize, packed: Packed, planes: [&mut [u8]; 3], plane_stride: usize, size: (usize, usize)) {
    use yuv::{YuvConversionMode, YuvRange, YuvStandardMatrix};
    let [y, u, v] = planes;
    let mut image = yuv::YuvPlanarImageMut {
        y_plane: yuv::BufferStoreMut::Borrowed(y),
        y_stride: plane_stride as u32,
        u_plane: yuv::BufferStoreMut::Borrowed(u),
        u_stride: plane_stride as u32,
        v_plane: yuv::BufferStoreMut::Borrowed(v),
        v_stride: plane_stride as u32,
        width: size.0 as u32,
        height: size.1 as u32,
    };
    let (range, matrix, mode) = (YuvRange::Limited, YuvStandardMatrix::Bt601, YuvConversionMode::Balanced);
    match packed {
        Packed::Rgb => yuv::rgb_to_yuv444(&mut image, pixels, stride as u32, range, matrix, mode),
        Packed::Bgrx => yuv::bgra_to_yuv444(&mut image, pixels, stride as u32, range, matrix, mode),
    }
    .expect("the picture fits its buffer, which was checked");
}

/// The byte order of a packed picture a [`Picture`] is read from.
#[derive(Clone, Copy)]
enum Packed {
    Rgb,
    Bgrx,
}

impl Packed {
    /// The bytes of one pixel.
    fn bytes(self) -> usize {
        match self {
            Self::Rgb => 3,
            Self::Bgrx => 4,
        }
    }
}

/// One VP9 stream at one picture size, coded by the [`Coder`] it was made
/// with. A picture of another size, or another coder, needs another encoder,
/// whose first frame is a keyframe by construction. Within one encoder, every
/// inter frame is expressed as a change from earlier frames, which is what
/// makes an inter-frame stream mean anything.
pub struct Encoder(Coded);

/// An [`Encoder`]'s coder.
enum Coded {
    /// Boxed, each of them: libvpx's holds its configuration and image in
    /// itself and the Rust coder its tables, hundreds of bytes and thousands,
    /// and a user moves an encoder to a worker and back for every frame.
    Libvpx(Box<libvpx::Encoder>),
    #[cfg(feature = "native")]
    Native(Box<native::Encoder>),
}

impl Encoder {
    /// An encoder for a `width`×`height` picture, starting at `quality`
    /// (1–100, clamped), coded by `coder` on `threads` threads. The thread
    /// count is the caller's: how many cores a machine can spare is a question
    /// about what else it runs. It also bounds the tile columns, and so
    /// decides with the width whether the stream is loop filtered: from 2048
    /// wide on four threads or more it is not.
    pub fn new(width: u16, height: u16, quality: u8, threads: usize, coder: Coder) -> Result<Self, Error> {
        Ok(Self(match coder {
            Coder::Libvpx => Coded::Libvpx(Box::new(libvpx::Encoder::new(width, height, quality, threads)?)),
            #[cfg(feature = "native")]
            Coder::Native => Coded::Native(Box::new(native::Encoder::new(width, height, quality, threads)?)),
        }))
    }

    /// The coder this encoder was made with.
    pub fn coder(&self) -> Coder {
        match &self.0 {
            Coded::Libvpx(_) => Coder::Libvpx,
            #[cfg(feature = "native")]
            Coded::Native(_) => Coder::Native,
        }
    }

    /// The picture size this encoder codes.
    pub fn size(&self) -> (u16, u16) {
        match &self.0 {
            Coded::Libvpx(encoder) => encoder.size(),
            #[cfg(feature = "native")]
            Coded::Native(encoder) => encoder.size(),
        }
    }

    /// The dial this encoder is coding at.
    pub fn quality(&self) -> u8 {
        match &self.0 {
            Coded::Libvpx(encoder) => encoder.quality(),
            #[cfg(feature = "native")]
            Coded::Native(encoder) => encoder.quality(),
        }
    }

    /// Move the dial, without a keyframe: the next frame is coded at the new
    /// quantizer against the frames before it. This is how a link that is
    /// behind gives quality up, and "without a keyframe" is the whole reason
    /// it is written this way rather than by rebuilding the encoder, which
    /// would force a keyframe — the most bytes a frame can be — at the exact
    /// moment the link has run out of room. It cannot fail: a libvpx that
    /// refuses the dial is the error of the frame that was to be coded at it.
    pub fn set_quality(&mut self, quality: u8) {
        match &mut self.0 {
            Coded::Libvpx(encoder) => encoder.set_quality(quality),
            #[cfg(feature = "native")]
            Coded::Native(encoder) => encoder.set_quality(quality),
        }
    }

    /// The coarsest dial any block of the picture a decoder now holds was last
    /// coded at: [`Self::quality`] as each frame was encoded, for the blocks
    /// that frame coded. A frame told where its picture changed leaves the
    /// rest at the dial they had, so a picture the dial was walked down for is
    /// coarse here until every block of it has been coded finer, which a whole
    /// frame does at once — what a caller that settles a quiet picture asks
    /// before deciding it owes one. Before the first frame, the dial.
    pub fn coarsest(&self) -> u8 {
        match &self.0 {
            Coded::Libvpx(encoder) => encoder.coarsest(),
            #[cfg(feature = "native")]
            Coded::Native(encoder) => encoder.coarsest(),
        }
    }

    /// Settle `picture` at `quality`: encode it whole and at that dial, and
    /// leave the dial where it was, for the frames after it. Returns whether
    /// the frame is a keyframe, as [`Self::encode`] does.
    pub fn settle(&mut self, picture: &Picture, quality: u8, keyframe: bool, out: &mut Vec<u8>) -> Result<bool, Error> {
        match &mut self.0 {
            Coded::Libvpx(encoder) => encoder.settle(picture, quality, keyframe, out),
            #[cfg(feature = "native")]
            Coded::Native(encoder) => encoder.settle(picture, quality, keyframe, out),
        }
    }

    /// Encode `picture` and append the frame to `out`. `keyframe` makes it one a
    /// decoder can start from; the first frame is one either way.
    ///
    /// `changed` is where the picture differs from the one encoded before it,
    /// or `None` for a picture that may differ anywhere. Blocks outside every
    /// rectangle are skipped — not searched, and left in the decoder's picture
    /// as they are — which is most of what a frame costs to encode when little
    /// moved. It is the caller's word and not checked: a change outside the
    /// rectangles is not coded, by this frame or by a later one that does not
    /// name it. A skipped block also keeps the quantizer it was last coded at,
    /// so the frame that is to sharpen an unchanged picture after
    /// [`Self::set_quality`] passes `None`. A keyframe codes everything
    /// whatever `changed` says.
    ///
    /// Returns whether the frame is a keyframe. On an error nothing is
    /// appended; libvpx making no frame of the picture is one
    /// ([`Error::NoFrame`]).
    pub fn encode(&mut self, picture: &Picture, keyframe: bool, changed: Option<&[Rect]>, out: &mut Vec<u8>) -> Result<bool, Error> {
        match &mut self.0 {
            Coded::Libvpx(encoder) => encoder.encode(picture, keyframe, changed, out),
            #[cfg(feature = "native")]
            Coded::Native(encoder) => encoder.encode(picture, keyframe, changed, out),
        }
    }
}

/// A desktop's stream from the pixels it is shown as: an [`Encoder`] and the
/// [`Picture`] in front of it, which is read from a packed picture where that
/// changed and then coded. What both users of this crate would otherwise each
/// keep: the rectangles that changed converted and the blocks they touch coded,
/// and the whole picture read and coded where there is nothing for it to be a
/// change to.
///
/// Reading and coding are two calls because they are not one moment to every
/// user: the pixels may be behind a lock the compositor waits on, which a
/// conversion is held for and an encode is not, and read straight from there
/// they are copied nowhere on the way.
pub struct Stream {
    encoder: Encoder,
    picture: Picture,
    /// The planes hold a picture: one was read whole. Until then there is
    /// nothing for rectangles to be read over, nor anything to code.
    filled: bool,
    /// What of the planes no decoder holds, which the next frame codes.
    owed: Owed,
    /// The rectangles read since the last frame, while that is all that is
    /// owed.
    changed: Vec<Rect>,
}

/// What of a [`Stream`]'s planes its decoder is yet to be sent.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owed {
    /// Nothing: the last frame carried the planes as they stand.
    Nothing,
    /// The rectangles read since the last frame.
    Changed,
    /// All of it: the picture was read whole, or a frame of it came to
    /// nothing.
    Whole,
}

impl Stream {
    /// A stream of `width`×`height` pictures, as [`Encoder::new`] takes them.
    pub fn new(width: u16, height: u16, quality: u8, threads: usize, coder: Coder) -> Result<Self, Error> {
        Ok(Self { encoder: Encoder::new(width, height, quality, threads, coder)?, picture: Picture::new(width, height, threads)?, filled: false, owed: Owed::Whole, changed: Vec::new() })
    }

    /// The coder this stream was made with.
    pub fn coder(&self) -> Coder {
        self.encoder.coder()
    }

    /// The picture size this stream codes.
    pub fn size(&self) -> (u16, u16) {
        self.encoder.size()
    }

    /// The dial this stream is coding at.
    pub fn quality(&self) -> u8 {
        self.encoder.quality()
    }

    /// Move the dial without a keyframe, as [`Encoder::set_quality`] does.
    pub fn set_quality(&mut self, quality: u8) {
        self.encoder.set_quality(quality);
    }

    /// The coarsest dial any block of the decoder's picture was last coded at
    /// ([`Encoder::coarsest`]).
    pub fn coarsest(&self) -> u8 {
        self.encoder.coarsest()
    }

    /// Read `pixels` — `B, G, R, X`, this stream's size, rows `stride` bytes
    /// apart — as the picture the next frame carries ([`Self::encode`]).
    ///
    /// `changed` is where the picture differs from the one read before it, or
    /// `None` for one that may differ anywhere, a keyframe's included: the
    /// caller's word and not checked. With it only the rectangles are read,
    /// and the next frame codes only the blocks they touch, and those of the
    /// reads before it that no frame has carried: the rest of `pixels` may be
    /// whatever it is. The stream's first read is the whole picture whatever
    /// `changed` says, and reads all of `pixels`.
    ///
    /// Nothing of `pixels` is kept: it is the planes that are coded, so the
    /// frame may be a while after the read, and the pixels changed by then.
    pub fn read_bgrx(&mut self, pixels: &[u8], stride: usize, changed: Option<&[Rect]>) -> Result<(), Error> {
        match changed.filter(|_| self.filled) {
            Some(rects) => {
                // Refused whole or not at all, a read of no rectangle too.
                self.picture.holds_bgrx(pixels, stride)?;
                for rect in rects {
                    self.picture.read_bgrx_rect(pixels, stride, *rect)?;
                }
                self.read(rects);
            }
            None => {
                self.picture.read_bgrx(pixels, stride)?;
                self.read_whole();
            }
        }
        Ok(())
    }

    /// Read `rgb` — packed RGB888, tight, for exactly this stream's picture —
    /// as [`Self::read_bgrx`] does `B, G, R, X`.
    pub fn read_rgb(&mut self, rgb: &[u8], changed: Option<&[Rect]>) -> Result<(), Error> {
        match changed.filter(|_| self.filled) {
            Some(rects) => {
                self.picture.holds_rgb(rgb)?;
                for rect in rects {
                    self.picture.read_rgb_rect(rgb, *rect)?;
                }
                self.read(rects);
            }
            None => {
                self.picture.read_rgb(rgb)?;
                self.read_whole();
            }
        }
        Ok(())
    }

    /// The planes were read at `rects`, which the next frame owes with what
    /// it owed already.
    fn read(&mut self, rects: &[Rect]) {
        match self.owed {
            Owed::Whole => {}
            Owed::Changed => self.changed.extend_from_slice(rects),
            Owed::Nothing => {
                self.changed.clear();
                self.changed.extend_from_slice(rects);
                self.owed = Owed::Changed;
            }
        }
    }

    /// The planes were read whole, and the next frame owes all of them.
    fn read_whole(&mut self) {
        self.filled = true;
        self.owed = Owed::Whole;
    }

    /// Encode the picture as it was last read and append the frame to `out`,
    /// returning what [`Encoder::encode`] does. The frame codes what was read
    /// since the frame before it: the blocks those rectangles touch, the whole
    /// picture if any read was whole, and no block at all if nothing was
    /// read, which is a frame all the same. `keyframe` makes it one a decoder
    /// can start from, which codes everything; the stream's first frame is
    /// one either way.
    ///
    /// A frame that failed, libvpx making nothing of it among them
    /// ([`Error::NoFrame`]), leaves the whole picture owed to the next, since
    /// no decoder was handed what this one coded. A stream nothing was ever
    /// read into has no picture to code, and says so ([`Error::Unread`]).
    pub fn encode(&mut self, keyframe: bool, out: &mut Vec<u8>) -> Result<bool, Error> {
        if !self.filled {
            return Err(Error::Unread);
        }
        let changed = match self.owed {
            Owed::Nothing => Some(&[][..]),
            Owed::Changed => Some(&self.changed[..]),
            Owed::Whole => None,
        };
        // Until a frame comes out the planes are ahead of every decoder.
        self.owed = Owed::Whole;
        let key = self.encoder.encode(&self.picture, keyframe, changed, out)?;
        self.owed = Owed::Nothing;
        Ok(key)
    }

    /// Settle the picture at `quality`: encode it as it was last read, whole
    /// and at that dial, and leave the dial where it was
    /// ([`Encoder::settle`]). What a desktop that went quiet while the link
    /// had it coarse is owed ([`walk::QualityWalk::settle_at`]): as an inter
    /// frame the unchanged picture costs no keyframe and sharpens every block,
    /// so [`Self::coarsest`] is `quality` after it, and the frames of the
    /// motion that follows are at what the link bears again. `keyframe` makes
    /// it one a decoder can start from, at `quality` all the same. Nothing
    /// need be read for it: a quiet desktop is the picture the planes hold,
    /// and one that changed since is read where it did ([`Self::read_bgrx`])
    /// first.
    pub fn settle(&mut self, quality: u8, keyframe: bool, out: &mut Vec<u8>) -> Result<bool, Error> {
        if !self.filled {
            return Err(Error::Unread);
        }
        self.owed = Owed::Whole;
        let key = self.encoder.settle(&self.picture, quality, keyframe, out)?;
        self.owed = Owed::Nothing;
        Ok(key)
    }
}

/// What the opening bits of a VP9 frame say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// The VP9 profile, 0–3. Streams encoded here are 1, 8-bit 4:4:4.
    pub profile: u8,
    /// Whether a decoder that has seen nothing before this frame can start here.
    pub keyframe: bool,
}

/// Read the first fields of a frame's uncompressed header (VP9 bitstream §6.2):
/// `frame_marker` (two bits, always 2), `profile_low_bit`, `profile_high_bit`, a
/// reserved zero bit on profile 3, `show_existing_frame`, and `frame_type`,
/// where 0 is a keyframe. A frame that only shows an earlier one is not a
/// keyframe.
///
/// For a stream this process did not encode, whose keyframe bit is therefore
/// not the encoder's to report. `None` for bytes that do not start a VP9 frame.
pub fn frame_header(frame: &[u8]) -> Option<FrameHeader> {
    let mut bits = frame.iter().take(2).flat_map(|byte| (0..8).rev().map(move |i| byte >> i & 1));
    let mut bit = || bits.next();
    if (bit()? << 1 | bit()?) != 2 {
        return None;
    }
    let low = bit()?;
    let profile = bit()? << 1 | low;
    if profile == 3 && bit()? != 0 {
        return None;
    }
    let show_existing = bit()? == 1;
    let keyframe = !show_existing && bit()? == 0;
    Some(FrameHeader { profile, keyframe })
}

/// VP9's levels, as `(level, max_luma_sample_rate, max_luma_picture_size,
/// max_luma_breadth)`.
///
/// Transcribed from `vp9_level_defs[]` in libvpx's own
/// `vp9/encoder/vp9_encoder.c` at the commit `libvpx-prebuilt` pins, and not
/// from memory: the table is the normative one, the rows are not evenly spaced,
/// and 4K30 lands on level 5.0 rather than the 5.1 a plausible guess gives. Only
/// the three fields a picture size decides are kept. A level also caps the
/// bitrate, the buffer size and the compression ratio, and a pinned quantizer
/// holds none of those: a busy frame costs what it costs, and it is the quality
/// walk behind the encoder, not a level, that keeps the stream inside the link.
/// So the level announced is the picture's, which is what a browser reads it
/// for: it picks a decoder by profile and size, and it refuses a string whose
/// level is not one of the fourteen — Chromium's parser rejects the "undefined"
/// 0 the file-format binding allows — so a level must be named.
///
/// The lowest row a picture fits is the level announced, which matters because
/// a level is a ceiling: a decoder that accepts 5.0 accepts every stream below
/// it, so the smallest level the picture fits is the widest claim to make.
const LEVELS: [(u8, u64, u32, u16); 14] = [
    (10, 829_440, 36_864, 512),
    (11, 2_764_800, 73_728, 768),
    (20, 4_608_000, 122_880, 960),
    (21, 9_216_000, 245_760, 1_344),
    (30, 20_736_000, 552_960, 2_048),
    (31, 36_864_000, 983_040, 2_752),
    (40, 83_558_400, 2_228_224, 4_160),
    (41, 160_432_128, 2_228_224, 4_160),
    (50, 311_951_360, 8_912_896, 8_384),
    (51, 588_251_136, 8_912_896, 8_384),
    (52, 1_176_502_272, 8_912_896, 8_384),
    (60, 1_176_502_272, 35_651_584, 16_832),
    (61, 2_353_004_544, 35_651_584, 16_832),
    (62, 4_706_009_088, 35_651_584, 16_832),
];

/// The WebCodecs codec string for a `w`×`h` stream of this crate's, every field
/// of it: `vp09.<profile>.<level>.<depth>.<chroma>.<primaries>.<transfer>.<matrix>.<range>`.
///
/// Profile 1, eight-bit and 4:4:4, because that is what the encoder produces.
/// The optional fields are spelled out rather than left to their
/// defaults because the defaults are wrong for this stream on both counts: an
/// omitted chroma field means 4:2:0, which Chromium reads as 4:2:2 on a profile
/// 1 string since 4:2:0 is not a profile 1 picture, and omitted colour fields
/// mean BT.709 where the keyframe header says BT.601 (SMPTE 170M primaries,
/// transfer and matrix — code 6 each, what Chromium's own VP9 parser maps that
/// header flag to) at studio swing. The level comes from `LEVELS` at `fps`
/// frames a second. `None` for a picture no VP9 level covers.
pub fn codec_string(w: u16, h: u16, fps: u64) -> Option<String> {
    let size = u32::from(w) * u32::from(h);
    let rate = u64::from(size) * fps;
    let breadth = w.max(h);
    let (level, ..) = LEVELS.iter().find(|(_, max_rate, max_size, max_breadth)| rate <= *max_rate && size <= *max_size && breadth <= *max_breadth).copied()?;
    Some(format!("vp09.01.{level:02}.08.03.06.06.06.00"))
}

// The crate's tests are made with the Rust coder: libvpx's own are beside it
// (`libvpx::tests`).
#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    // The Rust coder itself, which these tests are of: libvpx's are beside it
    // (`libvpx::tests`).
    use crate::native::Encoder;

    /// `w`×`h` of one colour, packed RGB.
    fn flat(w: u16, h: u16, colour: [u8; 3]) -> Vec<u8> {
        colour.iter().copied().cycle().take(usize::from(w) * usize::from(h) * 3).collect()
    }

    /// A stream is coded by the coder it was made with and by the same calls
    /// whichever that is: a keyframe first, a frame of what was read after
    /// it, and a settle at the dial it names, each a profile 1 frame the
    /// decoder takes.
    #[test]
    fn a_stream_is_coded_by_the_coder_it_names() {
        let (w, h) = (96u16, 64u16);
        for coder in [Coder::Libvpx, Coder::Native] {
            let mut stream = Stream::new(w, h, 40, 2, coder).expect("a stream");
            assert_eq!((stream.coder(), stream.size(), stream.quality()), (coder, (w, h), 40));
            let mut decoder = Decoder::new(1).expect("a decoder");
            let mut frame = Vec::new();
            assert!(matches!(stream.encode(false, &mut frame), Err(Error::Unread)), "{coder:?}");
            stream.read_rgb(&flat(w, h, [200, 40, 40]), None).expect("its own picture");
            assert!(stream.encode(false, &mut frame).expect("a frame"), "{coder:?}: a first frame that is no keyframe");
            assert_eq!(frame_header(&frame), Some(FrameHeader { profile: PROFILE, keyframe: true }), "{coder:?}");
            assert_eq!(decoder.decode(&frame).expect("a decode").size(), (u32::from(w), u32::from(h)), "{coder:?}");

            stream.set_quality(20);
            frame.clear();
            stream.read_rgb(&flat(w, h, [40, 200, 40]), Some(&[Rect { x: 0, y: 0, width: 32, height: 32 }])).expect("its own picture");
            assert!(!stream.encode(false, &mut frame).expect("a frame"), "{coder:?}: a keyframe nobody asked for");
            decoder.decode(&frame).expect("a decode");
            assert_eq!(stream.coarsest(), 20, "{coder:?}");

            frame.clear();
            assert!(!stream.settle(40, false, &mut frame).expect("a settle"), "{coder:?}: a settle that is a keyframe");
            decoder.decode(&frame).expect("a decode");
            assert_eq!((stream.coarsest(), stream.quality()), (40, 20), "{coder:?}");
        }
    }

    /// A picture read from `rgb`.
    fn picture(w: u16, h: u16, rgb: &[u8]) -> Picture {
        let mut picture = Picture::new(w, h, 1).expect("a picture");
        picture.read_rgb(rgb).expect("its own picture");
        picture
    }

    /// An encoder and the libvpx decoder following its stream, so that no
    /// frame of it goes undecoded whatever a test does with it after.
    struct Followed {
        encoder: Encoder,
        decoder: Decoder,
    }

    impl Followed {
        fn new(width: u16, height: u16, quality: u8, threads: usize) -> Self {
            Self { encoder: Encoder::new(width, height, quality, threads).expect("an encoder"), decoder: Decoder::new(1).expect("a decoder") }
        }
    }

    impl std::ops::Deref for Followed {
        type Target = Encoder;
        fn deref(&self) -> &Encoder {
            &self.encoder
        }
    }

    impl std::ops::DerefMut for Followed {
        fn deref_mut(&mut self) -> &mut Encoder {
            &mut self.encoder
        }
    }

    /// Encode `picture`, decode the frame, and return it with its keyframe
    /// bit.
    fn encode(encoder: &mut Followed, picture: &Picture, keyframe: bool) -> (Vec<u8>, bool) {
        let mut out = Vec::new();
        let key = encoder.encoder.encode(picture, keyframe, None, &mut out).expect("a frame");
        let (w, h) = picture.size();
        assert_eq!(encoder.decoder.decode(&out).expect("a frame libvpx decodes").size(), (u32::from(w), u32::from(h)));
        (out, key)
    }

    /// A chain of frames through one decoder, keyframe first, and the picture
    /// the last one leaves as `B, G, R, X` rows of `w` pixels.
    fn decode_chain(frames: &[Vec<u8>], w: usize, h: usize) -> Vec<u8> {
        let mut decoder = Decoder::new(1).expect("a decoder");
        let mut out = vec![0; w * h * 4];
        for frame in frames {
            let decoded = decoder.decode(frame).expect("a decode");
            assert_eq!(decoded.size(), (w as u32, h as u32));
            decoded.write_bgrx(&mut out, w * 4).expect("a buffer the picture fits");
        }
        out
    }

    /// The pixel at `(x, y)` of a decoded picture, as RGB.
    fn rgb_at(bgrx: &[u8], w: usize, x: usize, y: usize) -> [u8; 3] {
        let p = &bgrx[(y * w + x) * 4..][..3];
        [p[2], p[1], p[0]]
    }

    /// A dark terminal with one-pixel coloured glyph stems, each at an odd
    /// column: the picture a colour sample per pixel is coded for.
    fn stems(w: u16, h: u16) -> (Vec<u8>, Vec<(usize, usize)>) {
        let (w, h) = (usize::from(w), usize::from(h));
        let mut rgb = flat(w as u16, h as u16, [30, 30, 30]);
        let mut at = Vec::new();
        for y in (0..h).filter(|y| y % 2 == 0) {
            for x in (1..w).step_by(4) {
                rgb[(y * w + x) * 3..][..3].copy_from_slice(&[255, 121, 198]);
                at.push((x, y));
            }
        }
        (rgb, at)
    }

    /// A 320×240 picture with a block moved by `step`, so there is something for
    /// the quantizer to be coarse about — a stream of identical frames costs
    /// nearly nothing at any quality and would compare two zeroes.
    fn moving(step: u16) -> Vec<u8> {
        let mut rgb = flat(320, 240, [30, 60, 90]);
        let stride = 320 * 3;
        for row in 0..80 {
            let at = (usize::from(step) * 3 + row) * stride + usize::from(step) * 9;
            rgb[at..at + 300].fill(230);
        }
        rgb
    }

    /// A rectangle converts to what the whole picture's conversion holds
    /// there and leaves the rest alone: on an odd row or column, at the last
    /// row and column, in a buffer that ends with its last row, past the
    /// picture's edge, and on any number of threads, whose bands a picture
    /// this small is given by a rectangle the size of a desktop below.
    #[test]
    fn a_rectangle_converts_as_the_whole_picture_does() {
        let rect = |x, y, width, height| Rect { x, y, width, height };
        let noise = |len: usize, salt: u32| {
            let mut seed = salt;
            (0..len).map(|_| { seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345); (seed >> 16) as u8 }).collect::<Vec<u8>>()
        };
        for (width, height, rects) in [
            (37usize, 21usize, vec![rect(0, 0, 37, 1), rect(5, 3, 1, 1), rect(11, 5, 9, 7), rect(36, 20, 1, 1), rect(30, 19, 40, 40), rect(0, 0, 37, 21), rect(7, 7, 0, 3), rect(40, 30, 5, 5), rect(1, 20, 35, 1), rect(3, 0, 31, 21)]),
            (1203, 1001, vec![rect(0, 0, 1203, 1001), rect(1, 1, 1202, 1000), rect(101, 3, 1100, 997), rect(0, 500, 1203, 501)]),
        ] {
            let stride = width * 4 + 12;
            let (before, after) = (noise((height - 1) * stride + width * 4, 1), noise((height - 1) * stride + width * 4, 2));
            // Packed RGB, which is tight.
            let tight = |pixels: &[u8]| -> Vec<u8> { pixels.chunks(stride).flat_map(|row| row[..width * 4].chunks(4).flat_map(|px| [px[2], px[1], px[0]])).collect() };
            let (before_rgb, after_rgb) = (tight(&before), tight(&after));
            for threads in [1, 4] {
                let mut whole = Picture::new(width as u16, height as u16, 1).expect("a picture");
                for named in &rects {
                    // What changed is this rectangle and nothing else.
                    let (mut changed, mut changed_rgb) = (before.clone(), before_rgb.clone());
                    for row in usize::from(named.y)..(usize::from(named.y) + usize::from(named.height)).min(height) {
                        let cols = usize::from(named.x).min(width)..(usize::from(named.x) + usize::from(named.width)).min(width);
                        changed[row * stride..][cols.start * 4..cols.end * 4].copy_from_slice(&after[row * stride..][cols.start * 4..cols.end * 4]);
                        changed_rgb[row * width * 3..][cols.start * 3..cols.end * 3].copy_from_slice(&after_rgb[row * width * 3..][cols.start * 3..cols.end * 3]);
                    }
                    whole.read_bgrx(&changed, stride).expect("a picture that fits");
                    let mut partial = Picture::new(width as u16, height as u16, threads).expect("a picture");
                    partial.read_bgrx(&before, stride).expect("a picture that fits");
                    partial.read_bgrx_rect(&changed, stride, *named).expect("a picture that fits");
                    assert!(partial.planes() == whole.planes(), "{named:?} on {threads}: the planes are not the whole conversion's");

                    whole.read_rgb(&changed_rgb).expect("its own picture");
                    let mut partial = Picture::new(width as u16, height as u16, threads).expect("a picture");
                    partial.read_rgb(&before_rgb).expect("its own picture");
                    partial.read_rgb_rect(&changed_rgb, *named).expect("its own picture");
                    assert!(partial.planes() == whole.planes(), "RGB {named:?} on {threads}: the planes are not the whole conversion's");
                }
                assert!(matches!(whole.read_bgrx_rect(&before[1..], stride, rect(0, 0, 1, 1)), Err(Error::Buffer { .. })), "a buffer too short for the picture was read");
                assert!(matches!(whole.read_rgb_rect(&before_rgb[3..], rect(0, 0, 1, 1)), Err(Error::Crop(..))), "a buffer that is not the picture was read");
            }
        }
    }

    /// A stream reads the rectangles a frame's changes are and nothing else of
    /// the buffer, and leaves the rest of the decoder's picture as it was. Its
    /// first read has no picture to be changes to, and is the whole buffer
    /// whatever it is told. A frame codes what was read since the one before
    /// it, in however many reads, and a keyframe all of the planes.
    #[test]
    fn a_stream_reads_and_codes_only_what_a_frame_names() {
        let (w, h) = (96u16, 80u16);
        let (wu, hu) = (usize::from(w), usize::from(h));
        const BACK: [u8; 3] = [20, 40, 80];
        const LIT: [u8; 3] = [240, 240, 240];
        const JUNK: [u8; 3] = [170, 85, 170];
        // On an odd row and column, and not on the encoder's 8×8 grid.
        let named = Rect { x: 33, y: 17, width: 31, height: 15 };
        // `rgb` in the byte order a stream is fed: packed RGB, or `B, G, R, X`.
        let packed = |rgb: &[u8], bgrx: bool| -> Vec<u8> { if bgrx { rgb.chunks(3).flat_map(|px| [px[2], px[1], px[0], 0]).collect() } else { rgb.to_vec() } };
        let paint = |rgb: &mut [u8], rect: Rect, colour: [u8; 3]| {
            for y in usize::from(rect.y)..usize::from(rect.y + rect.height) {
                for x in usize::from(rect.x)..usize::from(rect.x + rect.width) {
                    rgb[(y * wu + x) * 3..][..3].copy_from_slice(&colour);
                }
            }
        };
        let near = |got: [u8; 3], want: [u8; 3]| (0..3).all(|c| got[c].abs_diff(want[c]) <= 40);
        for bgrx in [true, false] {
            let mut stream = Stream::new(w, h, 90, 2, Coder::Native).expect("a stream");
            assert_eq!((stream.size(), stream.quality(), stream.coarsest()), ((w, h), 90, 90));
            let mut frames = Vec::new();
            let mut step = |stream: &mut Stream, rgb: &[u8], changed: Option<&[Rect]>, keyframe: bool| {
                let pixels = packed(rgb, bgrx);
                let mut out = Vec::new();
                if bgrx { stream.read_bgrx(&pixels, wu * 4, changed) } else { stream.read_rgb(&pixels, changed) }.expect("a read");
                let key = stream.encode(keyframe, &mut out);
                frames.push(out);
                (key.expect("a frame"), decode_chain(&frames, wu, hu))
            };
            let whole = flat(w, h, BACK);

            // Nothing was read, so there is nothing to code or to settle.
            assert!(matches!(stream.encode(false, &mut Vec::new()), Err(Error::Unread)));
            assert!(matches!(stream.settle(90, false, &mut Vec::new()), Err(Error::Unread)));

            // The first frame is told of changes it cannot be changes to.
            let (key, back) = step(&mut stream, &whole, Some(&[named]), false);
            assert!(key && near(rgb_at(&back, wu, 4, 70), BACK) && near(rgb_at(&back, wu, 48, 24), BACK), "the first frame was not the whole picture");

            // A change, in a buffer the rest of which is not the picture.
            let mut rows = flat(w, h, JUNK);
            paint(&mut rows, named, LIT);
            let (key, back) = step(&mut stream, &rows, Some(&[named]), false);
            assert!(!key && near(rgb_at(&back, wu, 48, 24), LIT), "the change came back {:?}", rgb_at(&back, wu, 48, 24));
            for (x, y) in [(4, 4), (4, 24), (90, 24), (32, 24), (48, 4), (48, 15), (48, 16), (48, 32), (48, 70)] {
                assert!(near(rgb_at(&back, wu, x, y), BACK), "({x}, {y}), outside the change, came back {:?}", rgb_at(&back, wu, x, y));
            }

            // Two reads before a frame, which carries both: the rectangle put
            // back, and a band lit under it. Read from buffers that hold
            // nothing else, and are gone by the time the frame is coded.
            let band = Rect { x: 0, y: 64, width: w, height: 16 };
            for (rect, colour) in [(named, BACK), (band, LIT)] {
                let mut rows = flat(w, h, JUNK);
                paint(&mut rows, rect, colour);
                let pixels = packed(&rows, bgrx);
                if bgrx { stream.read_bgrx(&pixels, wu * 4, Some(&[rect])) } else { stream.read_rgb(&pixels, Some(&[rect])) }.expect("a read");
            }
            let mut out = Vec::new();
            assert!(!stream.encode(false, &mut out).expect("a frame"));
            frames.push(out);
            let back = decode_chain(&frames, wu, hu);
            assert!(near(rgb_at(&back, wu, 48, 24), BACK) && near(rgb_at(&back, wu, 48, 70), LIT) && near(rgb_at(&back, wu, 4, 4), BACK), "a frame did not carry both of its reads");

            // A frame of nothing read is a frame, and changes nothing.
            let mut out = Vec::new();
            assert!(!stream.encode(false, &mut out).expect("a frame"));
            frames.push(out);
            assert_eq!(decode_chain(&frames, wu, hu), back, "a frame of nothing read changed the picture");

            // A settle carries what was read and no frame has: the rectangle
            // lit again, with the rest as the planes hold it.
            let mut rows = flat(w, h, JUNK);
            paint(&mut rows, named, LIT);
            let pixels = packed(&rows, bgrx);
            if bgrx { stream.read_bgrx(&pixels, wu * 4, Some(&[named])) } else { stream.read_rgb(&pixels, Some(&[named])) }.expect("a read");
            let mut out = Vec::new();
            assert!(!stream.settle(90, false, &mut out).expect("a settle"));
            frames.push(out);
            let back = decode_chain(&frames, wu, hu);
            assert!(near(rgb_at(&back, wu, 48, 24), LIT) && near(rgb_at(&back, wu, 48, 70), LIT) && near(rgb_at(&back, wu, 4, 4), BACK), "a settle did not carry what was read for it");

            // A keyframe is the whole picture, of which it was read one
            // rectangle: the planes hold the rest.
            let mut rows = flat(w, h, JUNK);
            paint(&mut rows, named, BACK);
            let pixels = packed(&rows, bgrx);
            if bgrx { stream.read_bgrx(&pixels, wu * 4, Some(&[named])) } else { stream.read_rgb(&pixels, Some(&[named])) }.expect("a read");
            let mut out = Vec::new();
            assert!(stream.encode(true, &mut out).expect("a frame"));
            let back = decode_chain(&[out], wu, hu);
            assert!(near(rgb_at(&back, wu, 48, 24), BACK) && near(rgb_at(&back, wu, 48, 70), LIT) && near(rgb_at(&back, wu, 4, 4), BACK), "the keyframe was not the whole picture");

            // A buffer that is not the picture is refused whatever is to be
            // read of it, nothing included.
            for changed in [None, Some(&[named][..]), Some(&[][..])] {
                assert!(matches!(stream.read_rgb(&[0; 9], changed), Err(Error::Crop(..))));
                assert!(matches!(stream.read_bgrx(&[0; 9], 4, changed), Err(Error::Buffer { .. })));
            }
        }
    }

    #[test]
    fn the_quality_dial_maps_onto_the_encoders_whole_range() {
        assert_eq!(quality_to_q(QUALITY_MIN), Q_COARSEST);
        assert_eq!(quality_to_q(QUALITY_MAX), Q_FINEST);
        assert_eq!(quality_to_q(0), Q_COARSEST, "clamped, not wrapped");
        assert_eq!(quality_to_q(255), Q_FINEST);
        for quality in 0..=u8::MAX {
            let q = quality_to_q(quality);
            assert!((Q_FINEST..=Q_COARSEST).contains(&q), "quality {quality} gave q {q}, outside VP9's range");
        }
        // Monotone: a higher dial is never a coarser picture.
        for quality in 1..100u8 {
            assert!(quality_to_q(quality) >= quality_to_q(quality + 1));
        }
    }

    /// Tile columns follow the width — one under 1440, two from 1440 through
    /// 1080p, four from 2048 and at 4K and 5K, eight from 5760 — and never
    /// outnumber the threads.
    #[test]
    fn tile_columns_follow_the_width_and_never_outnumber_the_threads() {
        assert_eq!(tile_columns_log2(1280, 8), 0);
        assert_eq!(tile_columns_log2(1439, 8), 0);
        assert_eq!(tile_columns_log2(1440, 8), 1);
        assert_eq!(tile_columns_log2(1920, 8), 1);
        assert_eq!(tile_columns_log2(2047, 8), 1);
        assert_eq!(tile_columns_log2(2048, 8), 2);
        assert_eq!(tile_columns_log2(2560, 8), 2);
        assert_eq!(tile_columns_log2(2880, 8), 2);
        assert_eq!(tile_columns_log2(3840, 8), 2);
        assert_eq!(tile_columns_log2(5120, 8), 2);
        assert_eq!(tile_columns_log2(5760, 8), 3);
        assert_eq!(tile_columns_log2(5760, 4), 2);
        assert_eq!(tile_columns_log2(3840, 2), 1);
        assert_eq!(tile_columns_log2(3840, 1), 0);
        assert_eq!(tile_columns_log2(3840, 0), 0);
    }

    /// The loop filter level a frame's uncompressed header carries (VP9
    /// bitstream §6.2), for a frame of this crate's encoder: eight bits, shown,
    /// and after a keyframe predicted from frames of its own size.
    pub(crate) fn filter_level(frame: &[u8]) -> u32 {
        let mut bits = frame.iter().flat_map(|byte| (0..8).rev().map(move |i| u32::from(byte >> i & 1)));
        let mut read = |n: usize| (0..n).fold(0, |value, _| value << 1 | bits.next().expect("a whole header"));
        assert_eq!(read(2), 2, "frame_marker");
        let profile = read(1) | read(1) << 1;
        assert!(profile < 2, "an eight-bit profile");
        assert_eq!(read(1), 0, "show_existing_frame");
        let keyframe = read(1) == 0;
        assert_eq!((read(1), read(1)), (1, 0), "show_frame, error_resilient_mode");
        if keyframe {
            assert_eq!(read(24), 0x49_83_42, "frame_sync_code");
            assert_ne!(read(3), 7, "color_space is not sRGB");
            // color_range, and on profile 1 the subsampling and a reserved bit.
            read(if profile == 1 { 4 } else { 1 });
            read(32);
            assert_eq!(read(1), 0, "render_and_frame_size_different");
        } else {
            // reset_frame_context, refresh_frame_flags, and a reference and its
            // sign bias three times.
            read(2 + 8 + 12);
            assert_eq!(read(1), 1, "found_ref");
            assert_eq!(read(1), 0, "render_and_frame_size_different");
            // allow_high_precision_mv, then the interpolation filter: one bit
            // if a block chooses its own, two more if the frame does.
            read(1);
            if read(1) == 0 {
                read(2);
            }
        }
        // refresh_frame_context, frame_parallel_decoding_mode, frame_context_idx.
        read(4);
        read(6)
    }

    /// A stream is filtered unless it is in four tile columns, and one that is
    /// not says so in every frame's header and decodes as any other.
    #[test]
    fn the_loop_filter_is_left_out_in_four_tile_columns() {
        assert!(loop_filtered(tile_columns_log2(2047, 8)));
        assert!(!loop_filtered(tile_columns_log2(2048, 8)));
        assert!(!loop_filtered(tile_columns_log2(3840, 4)));
        assert!(loop_filtered(tile_columns_log2(3840, 3)));

        for (w, threads, filtered) in [(1440, 4, true), (2048, 4, false), (2048, 2, true)] {
            let mut encoder = Followed::new(w, 64, 50, threads);
            let frames: Vec<Vec<u8>> = [[30, 60, 90], [200, 30, 30], [40, 180, 70]].iter().map(|colour| encode(&mut encoder, &picture(w, 64, &flat(w, 64, *colour)), false).0).collect();
            for (n, frame) in frames.iter().enumerate() {
                assert_eq!(filter_level(frame) > 0, filtered, "frame {n} of {w} wide, {threads} threads");
            }
            let decoded = decode_chain(&frames, usize::from(w), 64);
            let last = rgb_at(&decoded, usize::from(w), usize::from(w) - 1, 63);
            assert!(last.iter().zip([40u8, 180, 70]).all(|(c, want)| c.abs_diff(want) <= 8), "{w} wide, {threads} threads decoded as {last:?}");
        }
    }

    #[test]
    fn a_picture_refuses_a_crop_that_is_not_its_picture() {
        let mut picture = Picture::new(64, 32, 1).expect("a picture");
        picture.read_rgb(&flat(64, 32, [10, 20, 30])).expect("its own picture");
        assert!(picture.read_rgb(&flat(64, 31, [10, 20, 30])).is_err(), "a mis-sized crop would have indexed out of the planes");
        // Three full-size planes, tight.
        let [y, u, v] = picture.planes();
        assert_eq!((y.len(), u.len(), v.len()), (64 * 32, 64 * 32, 64 * 32));
        assert_eq!(picture.strides(), [64, 64, 64]);
        assert!(matches!(Picture::new(0, 16, 1), Err(Error::Empty(0, 16))));
    }

    /// The conversion's arithmetic, at the points BT.601 studio swing pins
    /// exactly: black and white land on 16 and 235, every grey is chroma-neutral
    /// at 128, and a saturated red is the strongest V a swing this size has. The
    /// tolerance is one code value, which is the rounding the integer
    /// coefficients are allowed.
    #[test]
    fn the_conversion_is_bt601_studio_swing() {
        let mut yuv = Picture::new(2, 2, 1).expect("a picture");
        let close = |got: u8, want: u8, what: &str| {
            assert!(got.abs_diff(want) <= 1, "{what}: got {got}, wanted {want}");
        };
        for (colour, y_want, u_want, v_want, name) in [
            ([0u8, 0, 0], 16u8, 128u8, 128u8, "black"),
            ([255, 255, 255], 235, 128, 128, "white"),
            ([128, 128, 128], 126, 128, 128, "mid grey"),
            ([255, 0, 0], 81, 90, 240, "red"),
            ([0, 0, 255], 41, 240, 110, "blue"),
        ] {
            yuv.read_rgb(&flat(2, 2, colour)).expect("a 2x2 picture");
            let [y, u, v] = yuv.planes();
            close(y[0], y_want, name);
            close(u[0], u_want, name);
            close(v[0], v_want, name);
            // The same pixels as `B, G, R, X` land on the same planes.
            let bgrx: Vec<u8> = (0..4).flat_map(|_| [colour[2], colour[1], colour[0], 0x55]).collect();
            yuv.read_bgrx(&bgrx, 8).expect("a 2x2 picture");
            let [y, u, v] = yuv.planes();
            close(y[0], y_want, name);
            close(u[0], u_want, name);
            close(v[0], v_want, name);
        }
        // Each pixel keeps its own colour: a checkerboard of full red and full
        // blue is not averaged.
        let mut quad = Vec::new();
        quad.extend_from_slice(&[255, 0, 0, 0, 0, 255]);
        quad.extend_from_slice(&[0, 0, 255, 255, 0, 0]);
        yuv.read_rgb(&quad).expect("a 2x2 picture");
        let [_, u, v] = yuv.planes();
        close(u[0], 90, "red pixel U");
        close(v[0], 240, "red pixel V");
        close(u[1], 240, "blue pixel U");
        close(v[1], 110, "blue pixel V");
    }

    /// The two conversions are each other's inverse to within the rounding a
    /// studio-swing round trip costs, on every channel's full range, through a
    /// real encode at the finest quantizer.
    #[test]
    fn the_colour_conversion_round_trips_through_the_codec() {
        let colours: Vec<[u8; 4]> = (0..=255u8)
            .step_by(15)
            .flat_map(|r| (0..=255u8).step_by(51).flat_map(move |g| (0..=255u8).step_by(85).map(move |b| [b, g, r, 0])))
            .collect();
        // A colour to an 8×8, so that what is tested is the matrix and not how
        // the quantizer carries an edge at every pixel.
        let (width, height) = (colours.len() * 8, 8);
        let pixels: Vec<u8> = (0..height).flat_map(|_| colours.iter().flat_map(|colour| colour.iter().copied().cycle().take(32))).collect();
        let mut picture = Picture::new(width as u16, height as u16, 1).expect("a picture");
        picture.read_bgrx(&pixels, width * 4).expect("a picture that fits");
        assert!(picture.planes()[0].iter().all(|&y| (16..=235).contains(&y)), "luma stays in studio range");

        let mut encoder = Followed::new(width as u16, height as u16, QUALITY_MAX, 1);
        let (frame, _) = encode(&mut encoder, &picture, false);
        let back = decode_chain(&[frame], width, height);
        for (i, (want, got)) in pixels.as_chunks::<4>().0.iter().zip(back.as_chunks::<4>().0).enumerate() {
            // Studio swing folds 256 levels into 219 and the quantizer costs a
            // little more; a wrong matrix would be off by tens.
            for channel in 0..3 {
                assert!(want[channel].abs_diff(got[channel]) <= 6, "pixel {i}: {want:?} came back {got:?}");
            }
            assert_eq!(got[3], 0, "X is written as zero");
        }
    }

    /// A last row with nothing after it is converted like the rows before it,
    /// which have a stride's padding after them, and written back the same way.
    #[test]
    fn the_last_row_needs_no_padding_after_it() {
        let (width, height, stride) = (3usize, 3usize, 3 * 4 + 8);
        let colour = |row: usize| [40 * row as u8, 200 - 50 * row as u8, 90, 0];
        let mut pixels = vec![0xAA; (height - 1) * stride + width * 4];
        for row in 0..height {
            for x in 0..width {
                pixels[row * stride + x * 4..][..4].copy_from_slice(&colour(row));
            }
        }
        let mut picture = Picture::new(width as u16, height as u16, 1).expect("a picture");
        picture.read_bgrx(&pixels, stride).expect("a picture that fits");
        let mut encoder = Followed::new(width as u16, height as u16, QUALITY_MAX, 1);
        let (frame, _) = encode(&mut encoder, &picture, false);
        let mut decoder = Decoder::new(1).expect("a decoder");
        let mut back = vec![0xAA; (height - 1) * stride + width * 4];
        decoder.decode(&frame).expect("a decode").write_bgrx(&mut back, stride).expect("a buffer the picture fits");
        for row in 0..height {
            for x in 0..width {
                let got = &back[row * stride + x * 4..][..4];
                let want = colour(row);
                // What is tested is the stride, not the quantizer: the
                // finest one still costs a few code values on a 3×3 picture.
                assert!((0..3).all(|c| want[c].abs_diff(got[c]) <= 8) && got[3] == 0, "row {row}: {want:?} came back {got:?}");
            }
        }
        assert!(back[width * 4..stride].iter().all(|&b| b == 0xAA), "the padding is not written");
    }

    #[test]
    fn a_picture_that_does_not_fit_its_buffer_is_refused() {
        assert!(fits(4, 2, 16, 32));
        assert!(fits(4, 2, 20, 36), "the last row needs no padding after it");
        assert!(!fits(4, 2, 12, 32), "a stride shorter than a row");
        assert!(!fits(4, 2, 16, 31));
        assert!(fits(0, 0, 0, 0));
        assert!(!fits(4, 1, usize::MAX, 16), "a stride the conversion cannot carry, however the bytes add up");
        let mut picture = Picture::new(4, 2, 1).expect("a picture");
        assert!(matches!(picture.read_bgrx(&[0; 12], 16), Err(Error::Buffer { .. })));
    }

    #[test]
    fn a_lower_quality_makes_a_smaller_stream() {
        let bytes = |quality| {
            let mut encoder = Followed::new(320, 240, quality, 2);
            (0..5u16).map(|step| encode(&mut encoder, &picture(320, 240, &moving(step)), false).0.len()).sum::<usize>()
        };
        let (coarse, fine) = (bytes(5), bytes(90));
        assert!(coarse < fine, "quality 5 encoded {coarse} bytes and quality 90 encoded {fine}; the dial is not reaching the encoder");
    }

    #[test]
    fn the_first_frame_is_a_keyframe_and_another_can_be_asked_for() {
        let mut encoder = Followed::new(320, 240, 60, 2);
        let mut frame = |step, keyframe| encode(&mut encoder, &picture(320, 240, &moving(step)), keyframe);
        let first = frame(0, false);
        assert!(first.1, "a decoder has to be able to start somewhere");
        assert_eq!(frame_header(&first.0), Some(FrameHeader { profile: PROFILE, keyframe: true }));
        assert!(!frame(1, false).1, "an unasked-for keyframe is bytes for nothing");
        assert!(!frame(2, false).1);
        let asked = frame(3, true);
        assert!(asked.1, "the keyframe flag did not reach the encoder");
        assert_eq!(frame_header(&asked.0), Some(FrameHeader { profile: 1, keyframe: true }));
        // And it is not sticky: the frame after a forced keyframe is an ordinary one.
        let after = frame(4, false);
        assert!(!after.1);
        assert_eq!(frame_header(&after.0), Some(FrameHeader { profile: 1, keyframe: false }));
    }

    /// However long a stream runs, no keyframe comes that was not asked for.
    #[test]
    fn no_keyframe_comes_unasked() {
        let (w, h) = (64u16, 48u16);
        let mut encoder = Followed::new(w, h, 60, 1);
        let mut rgb = flat(w, h, [80, 40, 20]);
        for step in 0..300u32 {
            let x = (step * 3) as usize % 48;
            for y in 8..24 {
                rgb[(y * usize::from(w) + x) * 3..][..3].copy_from_slice(&[step as u8, (step * 7) as u8, (step * 13) as u8]);
            }
            let (frame, keyframe) = encode(&mut encoder, &picture(w, h, &rgb), false);
            assert_eq!(keyframe, step == 0, "frame {step}");
            assert_eq!(frame_header(&frame).map(|header| header.profile), Some(1), "frame {step}");
        }
    }

    #[test]
    fn a_frame_header_is_read_from_the_bitstream_alone() {
        assert_eq!(frame_header(&[]), None);
        assert_eq!(frame_header(&[0x00, 0x00]), None, "no frame marker");
    }

    /// A stream settles its picture with one frame: whole and at the quality
    /// asked whatever damage the frames before it were told, an inter frame
    /// unless a keyframe is asked, and with the dial back where the link had it
    /// for the frame after. A walk told what each frame left owes the settle
    /// until that one has gone.
    #[test]
    fn a_stream_settles_a_coarse_picture_and_keeps_its_dial() {
        let (w, h) = (320u16, 240u16);
        let (wu, hu) = (usize::from(w), usize::from(h));
        // Speckle, so a coarse quantizer has detail to lose.
        let mut rgb = flat(w, h, [240, 240, 240]);
        let mut seed = 12_345u32;
        for px in rgb.chunks_mut(3) {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            if (seed >> 16).is_multiple_of(5) {
                px.copy_from_slice(&[20, 20, 20]);
            }
        }
        let bgrx: Vec<u8> = rgb.chunks(3).flat_map(|px| [px[2], px[1], px[0], 0]).collect();
        let error = |back: &[u8]| {
            let sum: u64 = (0..hu).flat_map(|y| (0..wu).map(move |x| (x, y))).map(|(x, y)| u64::from(rgb_at(back, wu, x, y)[1].abs_diff(rgb[(y * wu + x) * 3 + 1]))).sum();
            sum as f64 / (wu * hu) as f64
        };
        let start = std::time::Instant::now();
        for packed_rgb in [false, true] {
            let mut walk = walk::QualityWalk::new(90, std::time::Duration::from_millis(33), true);
            let mut stream = Stream::new(w, h, 90, 2, Coder::Native).expect("a stream");
            stream.set_quality(QUALITY_MIN);
            let mut frames = vec![Vec::new()];
            if packed_rgb { stream.read_rgb(&rgb, None) } else { stream.read_bgrx(&bgrx, wu * 4, None) }.expect("a read");
            stream.encode(false, &mut frames[0]).expect("a frame");
            walk.sent(stream.coarsest(), start);
            assert_eq!((stream.coarsest(), walk.settle_at()), (QUALITY_MIN, Some(start + walk::SETTLE_IDLE)));
            let coarse = error(&decode_chain(&frames, wu, hu));

            walk.settle(start + walk::SETTLE_IDLE);
            let mut settle = Vec::new();
            // Nothing is read for it: the planes are the quiet desktop.
            assert!(!stream.settle(walk.ceiling(), false, &mut settle).expect("a settle"), "the settle cost a keyframe");
            frames.push(settle);
            walk.sent(stream.coarsest(), start + walk::SETTLE_IDLE);
            assert_eq!((stream.quality(), stream.coarsest(), walk.settle_at()), (QUALITY_MIN, 90, None), "the settle left the dial or the picture where it was not");
            let settled = error(&decode_chain(&frames, wu, hu));
            assert!(settled < coarse / 4.0, "the settle left the picture at error {settled:.2} of {coarse:.2}");

            // The motion after it is at what the link bears, and where it is.
            let mut moved = Vec::new();
            stream.read_rgb(&rgb, Some(&[Rect { x: 0, y: 0, width: 16, height: 16 }])).expect("a read");
            stream.encode(false, &mut moved).expect("a frame");
            frames.push(moved);
            walk.sent(stream.coarsest(), start + walk::SETTLE_IDLE);
            assert_eq!((stream.coarsest(), walk.settle_at().is_some()), (QUALITY_MIN, true), "a coarse frame after the settle owes none");

            // That frame coded its corner and left the rest as the settle
            // had it, and a frame codes what was read for it and not what
            // was read for the one before: a finer frame elsewhere leaves
            // the corner coarse, and a finer one of the corner leaves nothing.
            stream.set_quality(90);
            stream.read_rgb(&rgb, Some(&[Rect { x: 64, y: 64, width: 16, height: 16 }])).expect("a read");
            frames.push(Vec::new());
            stream.encode(false, frames.last_mut().expect("pushed above")).expect("a frame");
            assert_eq!(stream.coarsest(), QUALITY_MIN, "a frame coded what was read for the one before it");
            stream.read_rgb(&rgb, Some(&[Rect { x: 0, y: 0, width: 16, height: 16 }])).expect("a read");
            frames.push(Vec::new());
            stream.encode(false, frames.last_mut().expect("pushed above")).expect("a frame");
            assert_eq!(stream.coarsest(), 90, "a frame of one corner coded more than it");
            stream.set_quality(QUALITY_MIN);

            // A settle that is to be a keyframe is one, at the settle's quality.
            let mut key = Vec::new();
            assert!(stream.settle(90, true, &mut key).expect("a settle"));
            assert_eq!((stream.quality(), stream.coarsest(), frame_header(&key).map(|header| header.keyframe)), (QUALITY_MIN, 90, Some(true)));
            // Every frame of the stream decodes, in the order it was made.
            frames.push(key);
            decode_chain(&frames, wu, hu);

            assert_eq!(stream.quality(), QUALITY_MIN);
        }
    }

    /// The mechanism a quality walk rests on: quality can be given up
    /// mid-stream without spending a keyframe to do it.
    #[test]
    fn the_quality_moves_on_a_live_encoder_without_a_keyframe() {
        let mut encoder = Followed::new(320, 240, 90, 2);
        assert_eq!(encoder.quality(), 90);
        let frame = |encoder: &mut Followed, step| encode(encoder, &picture(320, 240, &moving(step)), false);
        frame(&mut encoder, 0);
        let fine: usize = (1..5).map(|step| frame(&mut encoder, step).0.len()).sum();

        encoder.set_quality(QUALITY_MIN);
        assert_eq!(encoder.quality(), QUALITY_MIN);
        let coarse: Vec<_> = (5..9).map(|step| frame(&mut encoder, step)).collect();
        assert!(coarse.iter().map(|(data, _)| data.len()).sum::<usize>() < fine, "the quantizer did not reach the running encoder");
        assert!(!coarse.iter().any(|(_, keyframe)| *keyframe), "moving the quantizer cost a keyframe, which is what this avoids");

        encoder.set_quality(0);
        assert_eq!(encoder.quality(), QUALITY_MIN, "clamped, not wrapped");
    }

    /// The speed reaches the running encoder without a keyframe, and the chain
    /// across the change still decodes to its picture.
    #[test]
    fn the_speed_moves_on_a_live_encoder_without_a_keyframe() {
        let mut encoder = Followed::new(320, 240, 90, 2);
        assert_eq!(encoder.speed(), Speed::Usual);
        let frame = |encoder: &mut Followed, step| encode(encoder, &picture(320, 240, &moving(step)), false);
        let mut frames = vec![frame(&mut encoder, 0).0];
        for speed in [Speed::Fastest, Speed::Usual] {
            encoder.set_speed(speed);
            assert_eq!(encoder.speed(), speed);
            for step in 1..4 {
                let (data, keyframe) = frame(&mut encoder, step);
                assert!(!keyframe, "moving the speed cost a keyframe");
                frames.push(data);
            }
        }
        let decoded = decode_chain(&frames, 320, 240);
        // Inside the block's last place, and the background far from it.
        let block = rgb_at(&decoded, 320, 3 * 3 + 50, 3 * 3 + 40);
        let ground = rgb_at(&decoded, 320, 300, 200);
        assert!(block.iter().all(|c| c.abs_diff(230) <= 8), "the block decoded as {block:?}");
        assert!(ground.iter().zip([30u8, 60, 90]).all(|(c, want)| c.abs_diff(want) <= 8), "the background decoded as {ground:?}");
    }

    /// An odd side is carried as it is and decodes at its own size, every
    /// sample to its last row and column within 3 levels of the planes the
    /// encoder was given: the finest quantizer still moves a flat block by a
    /// level or two.
    ///
    /// Judged on the planes rather than on RGB: the conversions either side of
    /// the codec are the `yuv` crate's, whose AVX2, NEON and scalar paths round
    /// differently, so a colour read back through them lands a CPU-dependent
    /// few levels further from where it started however well the codec did.
    #[test]
    fn an_odd_sized_picture_encodes_and_decodes_at_its_own_size() {
        let (w, h) = (33u16, 17u16);
        let source = picture(w, h, &flat(w, h, [40, 180, 90]));
        let mut encoder = Followed::new(w, h, QUALITY_MAX, 1);
        let (frame, _) = encode(&mut encoder, &source, false);
        let mut decoder = Decoder::new(1).expect("a decoder");
        let decoded = decoder.decode(&frame).expect("a decode");
        assert_eq!(decoded.size(), (33, 17));
        let (given, given_strides) = (source.planes(), source.strides());
        let (got, got_strides) = (decoded.planes(), decoded.strides());
        for plane in 0..3 {
            for (x, y) in (0..17).flat_map(|y| (0..33).map(move |x| (x, y))) {
                let (want, have) = (given[plane][y * given_strides[plane] + x], got[plane][y * got_strides[plane] + x]);
                assert!(want.abs_diff(have) <= 3, "plane {plane} at {x},{y}: {have}, given {want}");
            }
        }
        let mut out = vec![0; 33 * 17 * 4];
        decoded.write_bgrx(&mut out, 33 * 4).expect("a buffer the picture fits");
        // 1919×1079 as a desktop is, at the size the gateway once padded.
        let source = picture(1919, 1079, &flat(1919, 1079, [90, 90, 90]));
        let mut encoder = Followed::new(1919, 1079, 60, 2);
        let (frame, _) = encode(&mut encoder, &source, false);
        assert_eq!(Decoder::new(1).expect("a decoder").decode(&frame).expect("a decode").size(), (1919, 1079));
    }

    /// Only the frames that are asked to be keyframes are, and the ones between
    /// decode against them.
    #[test]
    fn keyframes_come_when_asked_and_the_frames_between_decode() {
        let (w, h) = (64u16, 32u16);
        let mut encoder = Followed::new(w, h, 60, 1);
        let mut decoder = Decoder::new(1).expect("a decoder");
        let mut out = vec![0; 64 * 32 * 4];
        for step in 0..5usize {
            let mut rgb = flat(w, h, [80, 40, 20]);
            for x in step * 4..step * 4 + 8 {
                for y in 0..32 {
                    rgb[(y * 64 + x) * 3..][..3].copy_from_slice(&[240, 240, 240]);
                }
            }
            let (frame, keyframe) = encode(&mut encoder, &picture(w, h, &rgb), step == 3);
            assert_eq!(keyframe, step == 0 || step == 3, "frame {step}");
            decoder.decode(&frame).expect("a decode").write_bgrx(&mut out, 64 * 4).expect("a buffer the picture fits");
            let lit = &out[(step * 4 + 4) * 4..][..3];
            assert!(lit.iter().all(|&c| c > 200), "frame {step} did not decode to its own picture: {lit:?}");
        }
    }

    /// The level table, at the rows a desktop actually lands on. The numbers are
    /// libvpx's own; what is tested is that the *lowest* fitting row is chosen,
    /// since a level is a ceiling and announcing a higher one narrows the set of
    /// decoders that will accept the stream.
    #[test]
    fn the_codec_string_names_the_lowest_level_that_fits() {
        let codec_string = |w, h| super::codec_string(w, h, 30);
        // 1280x800 at 30: 1_024_000 samples. Level 3.1 allows only 983_040 of
        // them, so this is level 4 — the *picture size* binds here, not the
        // sample rate, which at 30_720_000 is well inside 3.1's 36_864_000.
        assert_eq!(codec_string(1280, 800).as_deref(), Some("vp09.01.40.08.03.06.06.06.00"));
        // 1920x1080: 2_073_600 samples, inside level 4's picture size of
        // 2_228_224, and 62_208_000 per second inside its 83_558_400.
        assert_eq!(codec_string(1920, 1080).as_deref(), Some("vp09.01.40.08.03.06.06.06.00"));
        // 3840x2160: 8_294_400 samples — past 4.1's picture size, inside 5.0's
        // 8_912_896 — and 248_832_000 per second, inside 5.0's 311_951_360.
        assert_eq!(codec_string(3840, 2160).as_deref(), Some("vp09.01.50.08.03.06.06.06.00"));
        // 3840x2400: 9_216_000 samples, past every level 5's 8_912_896 picture
        // size by the 16:10 panel's extra 240 rows — level 6.0, the first that
        // holds it.
        assert_eq!(codec_string(3840, 2400).as_deref(), Some("vp09.01.60.08.03.06.06.06.00"));
        // Small, but not level 1: 76_800 samples is already past level 1.1's
        // 73_728. Level 1 is 256x144, which no desktop is.
        assert_eq!(codec_string(320, 240).as_deref(), Some("vp09.01.20.08.03.06.06.06.00"));
        // Profile 1, eight bits and 4:4:4 whatever the size, and the colour
        // fields never move: BT.601 at studio swing is what every keyframe
        // header declares, and a string that left them out would be read as
        // BT.709.
        for (w, h) in [(320u16, 240u16), (1920, 1080), (3840, 2160), (3840, 2400)] {
            let string = codec_string(w, h).expect("a level for a real desktop");
            assert!(string.starts_with("vp09.01."), "not profile 1: {string}");
            assert!(string.ends_with(".08.03.06.06.06.00"), "not 8-bit 4:4:4 BT.601: {string}");
        }
        // At 60 frames a second the sample rate binds where the picture size did
        // not: 1080p60 is 124_416_000 samples a second, past 4.0's 83_558_400,
        // and 4K60 497_664_000, past 5.0's 311_951_360.
        let at_60 = |w, h| super::codec_string(w, h, 60);
        assert_eq!(at_60(1920, 1080).as_deref(), Some("vp09.01.41.08.03.06.06.06.00"));
        assert_eq!(at_60(3840, 2160).as_deref(), Some("vp09.01.51.08.03.06.06.06.00"));
        assert_eq!(super::codec_string(20_000, 20_000, 60), None, "no level covers it");
    }

    /// **What a colour sample per pixel is for.** A one-pixel coloured glyph
    /// stem on a dark terminal comes back as it was at the dial's finest
    /// quantizer, where a stream that averaged its colour with the three
    /// background pixels beside it would return it at a fraction of its
    /// saturation at any quality.
    #[test]
    fn a_one_pixel_stem_keeps_its_colour() {
        let (rgb, at) = stems(64, 64);
        let mut encoder = Followed::new(64, 64, QUALITY_MAX, 1);
        let (frame, _) = encode(&mut encoder, &picture(64, 64, &rgb), false);
        let decoded = decode_chain(&[frame], 64, 64);
        let worst = at.iter().map(|&(x, y)| rgb_at(&decoded, 64, x, y).iter().zip([255u8, 121, 198]).map(|(a, b)| a.abs_diff(b)).max().unwrap()).max().unwrap();
        assert!(worst <= 24, "a stem pixel came back {worst} code values off at the finest quantizer");
    }

    /// The keyframe header says which matrix and range the pixels were converted
    /// with, so a decoder does not guess — and guesses BT.709 for an HD picture,
    /// which is not what the conversion did.
    #[test]
    fn the_bitstream_declares_bt601_studio_swing() {
        let mut encoder = Followed::new(64, 64, 60, 1);
        let (frame, _) = encode(&mut encoder, &picture(64, 64, &flat(64, 64, [200, 30, 30])), false);
        let mut decoder = Decoder::new(1).expect("a decoder");
        let decoded = decoder.decode(&frame).expect("a decode");
        assert!(decoded.declares_bt601_studio_swing());
        assert_eq!(decoded.planes()[1].len(), 63 * decoded.strides()[1] + 64);
    }

    #[test]
    fn a_wrong_picture_is_an_error_rather_than_a_read_past_the_end() {
        let mut encoder = Followed::new(32, 16, 60, 1);
        let other = picture(16, 16, &flat(16, 16, [0, 0, 0]));
        assert!(matches!(encoder.encode(&other, false, None, &mut Vec::new()), Err(Error::Mismatch(32, 16, 16, 16))));
        assert!(matches!(Encoder::new(0, 16, 60, 1), Err(Error::Empty(0, 16))));
        assert!(matches!(Encoder::new(16, 16, 60, 0), Err(Error::Threads(0))));

        let (frame, _) = encode(&mut encoder, &picture(32, 16, &flat(32, 16, [0, 0, 0])), false);
        let mut decoder = Decoder::new(1).expect("a decoder");
        assert!(decoder.decode(&[0xFF, 0x00, 0x12]).is_err());
        assert_eq!(decoder.decode(&frame).expect("a decode").size(), (32, 16));
    }

    /// The walk and a live encoder together, on a link that carries so many
    /// bytes a second: every move of the dial reaches the encoder without a
    /// keyframe, the frames answer it in size and in sharpness, an independent
    /// decoder takes every one of them, and the walk settles where the link
    /// keeps up and comes back to the ceiling once it widens.
    ///
    /// The picture is new every frame, as a video playing on the desktop is, so
    /// a frame's size answers to the quantizer; the link is half of what the
    /// ceiling costs, measured on an encoder of its own. Time is simulated —
    /// the walk takes `now` — so the run takes as long as its encodes.
    #[test]
    fn a_walk_moves_a_live_encoder_to_what_a_link_bears_and_back() {
        use std::collections::VecDeque;
        use std::time::{Duration, Instant};
        use walk::{Pace, QualityWalk, QUALITY_FLOOR};

        const W: u16 = 128;
        const H: u16 = 96;
        const CEILING: u8 = 90;
        /// 30 Hz.
        const FRAME: Duration = Duration::from_micros(33_333);
        /// Each way, so the link's floor is twice this and a frame's time on the
        /// wire: what the walk learns to read as distance rather than queueing.
        const ONE_WAY: Duration = Duration::from_millis(20);

        /// Noise new every frame.
        fn churning(step: u32) -> Picture {
            let rgb: Vec<u8> = (0..u32::from(W) * u32::from(H))
                .flat_map(|at| {
                    let v = ((at.wrapping_mul(2_654_435_761) ^ step.wrapping_mul(40_503)).wrapping_mul(2_246_822_519) >> 24) as u8;
                    [v, v / 2 + 40, 255 - v]
                })
                .collect();
            picture(W, H, &rgb)
        }

        /// A frame as it went out.
        struct Sent {
            quality: u8,
            bytes: usize,
            /// How long it waited behind the frames before it for the wire.
            queued: Duration,
            /// Mean distance of the decoded luma from the luma encoded.
            error: f64,
        }

        struct Session {
            encoder: Followed,
            decoder: Decoder,
            walk: QualityWalk,
            now: Instant,
            /// When the wire is next free.
            free: Instant,
            /// Fences on their way back: when each arrives, the delivery it
            /// reports, and whether it is a verdict.
            fences: VecDeque<(Instant, Duration, bool)>,
            step: u32,
        }

        impl Session {
            /// `span` of frames onto a link that carries `capacity` bytes a second.
            fn run(&mut self, capacity: f64, span: Duration) -> Vec<Sent> {
                let end = self.now + span;
                let mut sent = Vec::new();
                while self.now < end {
                    while let Some((back, delivery, verdict)) = self.fences.front().copied().filter(|(back, ..)| *back <= self.now) {
                        self.fences.pop_front();
                        if let Some(pace) = self.walk.fenced(delivery, verdict, back) {
                            self.encoder.set_quality(pace.quality);
                        }
                    }
                    assert_eq!(self.encoder.quality(), self.walk.quality(), "the encoder is not coding what the walk holds");

                    let source = churning(self.step);
                    let (frame, keyframe) = encode(&mut self.encoder, &source, false);
                    assert_eq!(keyframe, self.step == 0, "frame {}: a move of the dial cost a keyframe", self.step);
                    let decoded = self.decoder.decode(&frame).expect("a decode");
                    assert_eq!(decoded.size(), (u32::from(W), u32::from(H)));
                    let (given, stride) = (source.planes()[0], source.strides()[0]);
                    let (got, got_stride) = (decoded.planes()[0], decoded.strides()[0]);
                    let distance: u64 = (0..usize::from(H))
                        .flat_map(|y| (0..usize::from(W)).map(move |x| (x, y)))
                        .map(|(x, y)| u64::from(given[y * stride + x].abs_diff(got[y * got_stride + x])))
                        .sum();
                    if keyframe {
                        self.walk.keyframe(self.now);
                    }

                    let queued = self.free.saturating_duration_since(self.now);
                    self.free = self.free.max(self.now) + Duration::from_secs_f64(frame.len() as f64 / capacity);
                    let back = self.free + 2 * ONE_WAY;
                    self.fences.push_back((back, back - self.now, !keyframe));
                    sent.push(Sent { quality: self.encoder.quality(), bytes: frame.len(), queued, error: distance as f64 / f64::from(W) / f64::from(H) });
                    self.step += 1;
                    self.now += self.walk.interval();
                }
                sent
            }
        }

        // A second of delta frames at `quality`, on an encoder of its own.
        let per_second = |quality| {
            let mut encoder = Followed::new(W, H, quality, 2);
            (0..31).map(|step| encode(&mut encoder, &churning(step), false).0.len()).skip(1).sum::<usize>() as f64
        };
        let (ceiling, floor) = (per_second(CEILING), per_second(QUALITY_FLOOR));
        let narrow = ceiling / 2.0;
        assert!(floor < narrow / 2.0, "the floor costs {floor} bytes a second against a link of {narrow}: the picture does not answer to the quantizer");

        let start = Instant::now();
        let mut session = Session {
            encoder: Followed::new(W, H, CEILING, 2),
            decoder: Decoder::new(1).expect("a decoder"),
            walk: QualityWalk::new(CEILING, FRAME, true),
            now: start,
            free: start,
            fences: VecDeque::new(),
            step: 0,
        };
        let mean = |frames: &[Sent], of: fn(&Sent) -> f64| frames.iter().map(of).sum::<f64>() / frames.len() as f64;

        // Long enough to settle: the keyframe's hold, the queue the ceiling built
        // behind it drained, a step up refused and walked back.
        let behind = session.run(narrow, Duration::from_secs(25));
        let fine = &behind[1..31];
        let settled = &behind[behind.len() - 150..];
        let quality = session.walk.quality();
        assert!(QUALITY_FLOOR < quality && quality < CEILING, "the walk settled at {quality} on a link half the ceiling's width");
        assert!(!session.walk.slowed(), "the frames did not come back once the quality had been given up");
        assert!(settled.iter().all(|sent| sent.queued < walk::LAG_CLEAR), "the link is still behind five seconds from the end");
        let (fine_bytes, settled_bytes) = (mean(fine, |sent| sent.bytes as f64), mean(settled, |sent| sent.bytes as f64));
        assert!(settled_bytes < fine_bytes * 0.6, "frames at {quality} averaged {settled_bytes:.0} bytes against {fine_bytes:.0} at the ceiling");
        let (fine_error, settled_error) = (mean(fine, |sent| sent.error), mean(settled, |sent| sent.error));
        assert!(settled_error > fine_error * 2.0, "frames at {quality} decoded at error {settled_error:.2} against {fine_error:.2} at the ceiling");

        // Long enough for the hold on the quality the link refused to run out,
        // and the steps up from under it.
        let wide = session.run(ceiling * 4.0, Duration::from_secs(20));
        assert_eq!(session.walk.pace(), Pace { quality: CEILING, interval: FRAME });
        let back = &wide[wide.len() - 30..];
        assert!(back.iter().all(|sent| sent.quality == CEILING));
        let back_error = mean(back, |sent| sent.error);
        assert!(back_error < fine_error * 1.25, "back at the ceiling the frames decoded at error {back_error:.2}, against {fine_error:.2} before");
    }
}
