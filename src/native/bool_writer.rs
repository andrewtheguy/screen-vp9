//! VP9's boolean coder, the writing half: libvpx's `vpx_writer`, which the
//! compressed header and every tile are coded with.

/// Bools coded at 8-bit probabilities into a growing buffer.
pub(super) struct BoolWriter {
    buffer: Vec<u8>,
    state: State,
}

/// The coder's low end and range. `low` holds the bits of the low end not
/// yet written, `width` of them, under at most one carry into the bytes
/// already written; it reaches the buffer four bytes at a time.
#[derive(Clone, Copy)]
struct State {
    low: u64,
    width: u32,
    range: u32,
}

impl State {
    #[inline]
    fn write(&mut self, buffer: &mut Vec<u8>, bit: bool, probability: u8) {
        let split = 1 + (((self.range - 1) * u32::from(probability)) >> 8);
        let (low, range) = if bit { (self.low + u64::from(split), self.range - split) } else { (self.low, split) };
        // Back to 128 or more. The range is 1 to 255, its top bit's place 0
        // to 7, and 7 less that is the place with its three bits flipped.
        let shift = 7 ^ range.ilog2();
        self.low = low << shift;
        self.range = range << shift;
        self.width += shift;
        // A shift is at most 7, so `low` holds at most 55 bits and the carry
        // above them before a write and 62 after.
        if self.width > 55 {
            (self.low, self.width) = flush(buffer, self.low, self.width, 4);
        }
    }
}

/// Write the top `bytes` bytes of `low`, `width` bits under a carry, first
/// carrying into those already written, and give back what is left.
#[cold]
#[inline(never)]
fn flush(buffer: &mut Vec<u8>, low: u64, width: u32, bytes: u32) -> (u64, u32) {
    if low >> width != 0 {
        // A carry runs back through the bytes already written: a coder's
        // interval only narrows, so there is one at most, and there is a
        // byte for it, the interval having started inside the first.
        let last = buffer.iter().rposition(|&b| b != 0xff).expect("a byte the carry stops at");
        buffer[last] += 1;
        buffer[last + 1..].fill(0);
    }
    let width = width - 8 * bytes;
    buffer.extend_from_slice(&(low >> width).to_be_bytes()[(8 - bytes) as usize..]);
    (low & ((1 << width) - 1), width)
}

impl BoolWriter {
    /// A writer that has written the marker bit a decoder reads first and
    /// requires to be zero.
    pub(super) fn new() -> Self {
        let mut w = Self { buffer: Vec::new(), state: State { low: 0, width: 8, range: 255 } };
        w.bit(false);
        w
    }

    /// Code `bit`, whose chance of being false is `probability` / 256.
    pub(super) fn write(&mut self, bit: bool, probability: u8) {
        self.state.write(&mut self.buffer, bit, probability);
    }

    /// Code `bit` at even odds.
    pub(super) fn bit(&mut self, bit: bool) {
        self.write(bit, 128);
    }

    /// Code the low `bits` bits of `value` at even odds, most significant first.
    pub(super) fn literal(&mut self, value: u32, bits: u32) {
        for i in (0..bits).rev() {
            self.bit(value >> i & 1 == 1);
        }
    }

    /// The writer as a run of bools whose state a loop can hold in
    /// registers, back in the writer when the run is dropped. A loop coding
    /// through the writer itself loads and stores the state at every bool,
    /// growing the buffer being able to reach it.
    pub(super) fn run(&mut self) -> Run<'_> {
        Run { state: self.state, buffer: &mut self.buffer, home: &mut self.state }
    }

    /// Flush what is held and give back the bytes: 32 zero bits, then
    /// every whole byte above the last 24 bits, as libvpx's writer leaves
    /// them. The last byte never looks like the marker of a superframe
    /// index, which a decoder looks for at the end of a frame.
    pub(super) fn finish(mut self) -> Vec<u8> {
        for _ in 0..32 {
            self.bit(false);
        }
        let State { low, width, .. } = self.state;
        flush(&mut self.buffer, low, width, (width - 24) / 8);
        if self.buffer.last().is_some_and(|&b| b & 0xe0 == 0xc0) {
            self.buffer.push(0);
        }
        self.buffer
    }
}

/// `BoolWriter::run`.
pub(super) struct Run<'a> {
    state: State,
    buffer: &'a mut Vec<u8>,
    home: &'a mut State,
}

impl Run<'_> {
    /// `BoolWriter::write`.
    #[inline]
    pub(super) fn write(&mut self, bit: bool, probability: u8) {
        self.state.write(self.buffer, bit, probability);
    }
}

impl Drop for Run<'_> {
    fn drop(&mut self) {
        *self.home = self.state;
    }
}

/// What coding `bit` at `probability` costs, in 256ths of a bit.
pub(super) fn cost(bit: bool, probability: u8) -> u32 {
    /// Per chance in 256 of what is coded, its cost.
    static COSTS: std::sync::LazyLock<[u32; 257]> = std::sync::LazyLock::new(|| std::array::from_fn(|p| (-(p as f64 / 256.0).log2() * 256.0).round() as u32));
    COSTS[if bit { 256 - usize::from(probability) } else { usize::from(probability) }]
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The reading half as the VP9 specification writes it (§9.2), apart
    /// from libvpx's, so that the writer is checked against the format
    /// rather than against its own source.
    pub(in crate::native) struct BoolReader<'a> {
        data: &'a [u8],
        bit: usize,
        value: u32,
        range: u32,
        max_bits: usize,
    }

    impl<'a> BoolReader<'a> {
        pub(in crate::native) fn new(data: &'a [u8]) -> Self {
            let mut r = Self { data, bit: 8, value: u32::from(data[0]), range: 255, max_bits: 8 * data.len() - 8 };
            assert!(!r.read(128), "the marker bit is zero");
            r
        }

        pub(in crate::native) fn read(&mut self, probability: u8) -> bool {
            let split = 1 + (((self.range - 1) * u32::from(probability)) >> 8);
            let bit = if self.value < split {
                self.range = split;
                false
            } else {
                self.range -= split;
                self.value -= split;
                true
            };
            while self.range < 128 {
                let next = if self.max_bits > 0 {
                    self.max_bits -= 1;
                    let b = self.data[self.bit / 8] >> (7 - self.bit % 8) & 1;
                    self.bit += 1;
                    b
                } else {
                    0
                };
                self.range *= 2;
                self.value = (self.value << 1) | u32::from(next);
            }
            bit
        }

        pub(in crate::native) fn literal(&mut self, bits: u32) -> u32 {
            (0..bits).fold(0, |v, _| v << 1 | u32::from(self.read(128)))
        }
    }

    /// A deterministic stream of bits and probabilities covering the whole
    /// range, carries included.
    fn sequence(n: usize) -> Vec<(bool, u8)> {
        let mut state = 0x2545_f491_u32;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let probability = (state >> 8) as u8;
                // Bits mostly the likely way, so long runs push the carry.
                let likely = state & 0xf != 0;
                (if likely { probability < 128 } else { probability >= 128 }, probability.max(1))
            })
            .collect()
    }

    #[test]
    fn what_is_written_reads_back() {
        for n in [0, 1, 7, 100, 10_000] {
            let bits = sequence(n);
            let mut w = BoolWriter::new();
            for &(bit, p) in &bits {
                w.write(bit, p);
            }
            let data = w.finish();
            let mut r = BoolReader::new(&data);
            for (i, &(bit, p)) in bits.iter().enumerate() {
                assert_eq!(r.read(p), bit, "bit {i} of {n}");
            }
        }
    }

    #[test]
    fn a_run_of_certain_bits_carries_through_written_bytes() {
        // Bits against a probability of 1 drive `low` up at every write.
        let mut w = BoolWriter::new();
        for _ in 0..5_000 {
            w.write(true, 1);
        }
        w.literal(0x2d, 6);
        let data = w.finish();
        let mut r = BoolReader::new(&data);
        assert!((0..5_000).all(|_| r.read(1)));
        assert_eq!(r.literal(6), 0x2d);
    }

    #[test]
    fn the_last_byte_is_never_a_superframe_marker() {
        for n in 0..200 {
            let mut w = BoolWriter::new();
            for &(bit, p) in &sequence(n) {
                w.write(bit, p);
            }
            let data = w.finish();
            assert_ne!(data.last().unwrap() & 0xe0, 0xc0, "{n} bools");
        }
    }
}
