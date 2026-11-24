//! Deterministic input generation.
//!
//! SplitMix64 (Steele, Lea and Flood, "Fast splittable pseudorandom number
//! generators", OOPSLA 2014), seeded from a label so that every oracle and
//! purpose gets its own reproducible stream. This is for test inputs only;
//! it is not a cryptographic generator and needs not be one.

/// A SplitMix64 generator.
#[derive(Clone, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// Seeds from a label, e.g. `"MLKEM-T01-ntt/reference-intermediate"`,
    /// using FNV-1a (64-bit) over the label bytes.
    #[must_use]
    pub fn from_label(label: &str) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in label.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self(h)
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// 32 bytes: four outputs, each little-endian.
    pub fn bytes32(&mut self) -> [u8; 32] {
        let mut b = [0u8; 32];
        for chunk in b.chunks_mut(8) {
            chunk.copy_from_slice(&self.next_u64().to_le_bytes());
        }
        b
    }

    /// A value in [0, n), by rejection (no modulo bias).
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let v = self.next_u64();
            if v < zone {
                return v % n;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First outputs of SplitMix64 seeded with 0. Expected values computed
    /// independently with a Python transcription of the algorithm:
    /// `python3 -c 'M=(1<<64)-1; s=0` then, three times,
    /// `s=(s+0x9e3779b97f4a7c15)&M; z=s; z=((z^(z>>30))*0xbf58476d1ce4e5b9)&M;
    /// z=((z^(z>>27))*0x94d049bb133111eb)&M; print(hex(z^(z>>31)))`.
    #[test]
    fn matches_reference_outputs_for_seed_zero() {
        let mut g = SplitMix64(0);
        assert_eq!(g.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(g.next_u64(), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(g.next_u64(), 0x06c4_5d18_8009_454f);
    }

    #[test]
    fn labels_give_different_streams() {
        let a = SplitMix64::from_label("a").next_u64();
        let b = SplitMix64::from_label("b").next_u64();
        assert_ne!(a, b);
    }

    #[test]
    fn below_stays_in_range() {
        let mut g = SplitMix64::from_label("range");
        for _ in 0..10_000 {
            assert!(g.below(3329) < 3329);
        }
    }
}
