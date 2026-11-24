//! FIPS 203 (ML-KEM) functions used by the pilot tasks.

/// The ML-KEM modulus q (FIPS 203 section 8, Table 2).
pub const Q: u32 = 3329;

/// The primitive 256-th root of unity zeta = 17 (FIPS 203 section 4.3).
pub const ZETA: u32 = 17;

/// The compression widths used by ML-KEM: d = 1 for messages, (du, dv) =
/// (10, 4) for ML-KEM-512/768 and (11, 5) for ML-KEM-1024 (Table 2).
pub const COMPRESS_D: [u32; 5] = [1, 4, 5, 10, 11];

fn pow_mod(base: u32, exp: u32) -> u32 {
    let mut result: u64 = 1;
    let mut b = u64::from(base % Q);
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result * b % u64::from(Q);
        }
        b = b * b % u64::from(Q);
        e >>= 1;
    }
    u32::try_from(result).expect("value below q fits in u32")
}

fn mul_mod(a: u32, b: u32) -> u32 {
    u32::try_from(u64::from(a) * u64::from(b) % u64::from(Q)).expect("below q")
}

/// BitRev7: the integer whose 7-bit binary form is that of `i` reversed
/// (FIPS 203 section 4.3).
///
/// # Panics
/// If `i >= 128`.
#[must_use]
pub fn bitrev7(i: u8) -> u8 {
    assert!(i < 128, "BitRev7 is defined on 0..=127");
    i.reverse_bits() >> 1
}

/// NTT, FIPS 203 Algorithm 9. Input and output coefficients are in [0, q).
///
/// # Panics
/// If an input coefficient is not in [0, q).
#[must_use]
pub fn ntt(f: &[u16; 256]) -> [u16; 256] {
    let mut fh: [u32; 256] = f.map(|c| {
        assert!(u32::from(c) < Q, "NTT input must be in [0, q)");
        u32::from(c)
    });
    let mut i: u8 = 1;
    let mut len = 128;
    while len >= 2 {
        let mut start = 0;
        while start < 256 {
            let zeta = pow_mod(ZETA, u32::from(bitrev7(i)));
            i += 1;
            for j in start..start + len {
                let t = mul_mod(zeta, fh[j + len]);
                fh[j + len] = (fh[j] + Q - t) % Q;
                fh[j] = (fh[j] + t) % Q;
            }
            start += 2 * len;
        }
        len /= 2;
    }
    fh.map(|c| u16::try_from(c).expect("below q"))
}

/// The NTT representation computed directly from its definition, FIPS 203
/// Eqs. (4.12) and (4.13): entry i is f mod (X^2 - zeta^(2 BitRev7(i) + 1)).
/// Used only to check [`ntt`]; it shares no code with the butterfly loop.
///
/// # Panics
/// Never: every value is reduced below q before conversion.
#[must_use]
pub fn ntt_by_definition(f: &[u16; 256]) -> [u16; 256] {
    let mut out = [0u16; 256];
    for i in 0..128u8 {
        let gamma = pow_mod(ZETA, 2 * u32::from(bitrev7(i)) + 1);
        // X^2 = gamma, so X^j = gamma^(j/2) * X^(j mod 2).
        let (mut even, mut odd) = (0u32, 0u32);
        let mut g = 1u32; // gamma^(j/2) for the current pair (2k, 2k+1)
        for k in 0..128 {
            even = (even + mul_mod(u32::from(f[2 * k]), g)) % Q;
            odd = (odd + mul_mod(u32::from(f[2 * k + 1]), g)) % Q;
            g = mul_mod(g, gamma);
        }
        let idx = 2 * usize::from(i);
        out[idx] = u16::try_from(even).expect("below q");
        out[idx + 1] = u16::try_from(odd).expect("below q");
    }
    out
}

/// Compress_d, FIPS 203 Eq. (4.7): round((2^d / q) * x) mod 2^d, with rounding
/// to the nearest integer and ties rounded up (FIPS 203 section 2.3).
///
/// Computed exactly in integers: round(a / b) = floor((2a + b) / 2b).
///
/// # Panics
/// If `d >= 12` or `x >= q`.
#[must_use]
pub fn compress(d: u32, x: u32) -> u32 {
    assert!(d < 12 && x < Q, "Compress_d needs d < 12 and x in [0, q)");
    let num = 2 * (x << d) + Q;
    (num / (2 * Q)) % (1 << d)
}

/// Decompress_d, FIPS 203 Eq. (4.8): round((q / 2^d) * y).
///
/// # Panics
/// If `d >= 12` or `y >= 2^d`.
#[must_use]
pub fn decompress(d: u32, y: u32) -> u32 {
    assert!(
        d < 12 && y < (1 << d),
        "Decompress_d needs d < 12 and y < 2^d"
    );
    (2 * Q * y + (1 << d)) / (1 << (d + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeta_is_a_primitive_256th_root_of_unity() {
        // FIPS 203 section 4.3: zeta^128 = -1 mod q.
        assert_eq!(pow_mod(ZETA, 128), Q - 1);
        assert_eq!(pow_mod(ZETA, 256), 1);
    }

    #[test]
    fn bitrev7_is_an_involution() {
        for i in 0..128u8 {
            assert_eq!(bitrev7(bitrev7(i)), i);
        }
        assert_eq!(bitrev7(1), 64);
        assert_eq!(bitrev7(127), 127);
    }

    /// The NTT is linear, so agreement on all 256 unit vectors (scaled by
    /// every coefficient value would be redundant) proves Algorithm 9 equals
    /// the definition (4.12)-(4.13) on every input.
    #[test]
    fn ntt_matches_definition_on_every_unit_vector() {
        for j in 0..256 {
            let mut f = [0u16; 256];
            f[j] = 1;
            assert_eq!(ntt(&f), ntt_by_definition(&f), "unit vector {j}");
        }
    }

    #[test]
    fn ntt_matches_definition_on_extremes() {
        let all_max = [u16::try_from(Q - 1).unwrap(); 256];
        assert_eq!(ntt(&all_max), ntt_by_definition(&all_max));
        assert_eq!(ntt(&[0; 256]), [0; 256]);
    }

    /// FIPS 203 section 4.2.1: Compress_d(Decompress_d(y)) = y for all y and
    /// all d < 12. Checked exhaustively for every d below 12.
    #[test]
    fn compress_inverts_decompress_for_all_d_below_12() {
        for d in 1..12 {
            for y in 0..(1u32 << d) {
                assert_eq!(compress(d, decompress(d, y)), y, "d={d} y={y}");
            }
        }
    }

    /// Compress_d against its definition computed with exact rationals
    /// (numerator and denominator), for every x in [0, q) and each ML-KEM d.
    #[test]
    fn compress_matches_rational_definition_exhaustively() {
        for d in COMPRESS_D {
            for x in 0..Q {
                // v = 2^d * x / q. round(v) = floor(v) + 1 if the remainder
                // is at least q/2 (ties up); q is odd, so 2*rem >= q.
                let n = x << d;
                let (fl, rem) = (n / Q, n % Q);
                let rounded = if 2 * rem >= Q { fl + 1 } else { fl };
                assert_eq!(compress(d, x), rounded % (1 << d), "d={d} x={x}");
            }
        }
    }

    #[test]
    fn compress_output_range() {
        for d in COMPRESS_D {
            for x in 0..Q {
                assert!(compress(d, x) < (1 << d));
            }
        }
    }
}
