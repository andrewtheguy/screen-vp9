//! VP9's boolean coder, the writing half: libvpx's `vpx_writer`, which the
//! compressed header and every tile are coded with.

/// The shift that brings a range of 1–255 back to 128 or more.
fn norm(range: u32) -> u32 {
    range.leading_zeros() - 24
}

/// Bools coded at 8-bit probabilities into a growing buffer.
pub(super) struct BoolWriter {
    buffer: Vec<u8>,
    low: u32,
    range: u32,
    count: i32,
}

impl BoolWriter {
    /// A writer that has written the marker bit a decoder reads first and
    /// requires to be zero.
    pub(super) fn new() -> Self {
        let mut w = Self { buffer: Vec::new(), low: 0, range: 255, count: -24 };
        w.bit(false);
        w
    }

    /// Code `bit`, whose chance of being false is `probability` / 256.
    pub(super) fn write(&mut self, bit: bool, probability: u8) {
        let split = 1 + (((self.range - 1) * u32::from(probability)) >> 8);
        let mut range = split;
        let mut low = self.low;
        if bit {
            low += split;
            range = self.range - split;
        }
        let mut shift = norm(range) as i32;
        range <<= shift;
        self.count += shift;
        if self.count >= 0 {
            let offset = shift - self.count;
            // A carry out of the 24 bits held runs back through the bytes
            // already written.
            if (low << (offset - 1)) & 0x8000_0000 != 0 {
                let mut x = self.buffer.len();
                while x > 0 && self.buffer[x - 1] == 0xff {
                    self.buffer[x - 1] = 0;
                    x -= 1;
                }
                self.buffer[x - 1] += 1;
            }
            self.buffer.push((low >> (24 - offset)) as u8);
            low <<= offset;
            shift = self.count;
            low &= 0xff_ffff;
            self.count -= 8;
        }
        self.low = low << shift;
        self.range = range;
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

    /// Flush what is held and give back the bytes. The last byte never looks
    /// like the marker of a superframe index, which a decoder looks for at the
    /// end of a frame.
    pub(super) fn finish(mut self) -> Vec<u8> {
        for _ in 0..32 {
            self.bit(false);
        }
        if self.buffer.last().is_some_and(|&b| b & 0xe0 == 0xc0) {
            self.buffer.push(0);
        }
        self.buffer
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The reading half as the VP9 specification writes it (§9.2), apart
    /// from libvpx's, so that the writer is checked against the format
    /// rather than against its own source.
    pub(in crate::encoder) struct BoolReader<'a> {
        data: &'a [u8],
        bit: usize,
        value: u32,
        range: u32,
        max_bits: usize,
    }

    impl<'a> BoolReader<'a> {
        pub(in crate::encoder) fn new(data: &'a [u8]) -> Self {
            let mut r = Self { data, bit: 8, value: u32::from(data[0]), range: 255, max_bits: 8 * data.len() - 8 };
            assert!(!r.read(128), "the marker bit is zero");
            r
        }

        pub(in crate::encoder) fn read(&mut self, probability: u8) -> bool {
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

        pub(in crate::encoder) fn literal(&mut self, bits: u32) -> u32 {
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
