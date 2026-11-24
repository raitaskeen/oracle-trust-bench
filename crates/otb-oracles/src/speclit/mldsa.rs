//! FIPS 204 (ML-DSA) functions used by the pilot tasks.

/// The ML-DSA modulus q (FIPS 204 section 4, Table 1).
pub const Q: i64 = 8_380_417;

/// Dropped bits from t, d = 13 (Table 1).
pub const D: u32 = 13;

/// gamma2 for ML-DSA-65 and ML-DSA-87, (q - 1) / 32 (Table 1).
pub const GAMMA2_65: i64 = (Q - 1) / 32;

/// m mod+ alpha: the representative in [0, alpha).
fn mod_plus(m: i64, alpha: i64) -> i64 {
    m.rem_euclid(alpha)
}

/// m mod± alpha: the representative in (-alpha/2, alpha/2] for even alpha
/// (FIPS 204 section 2.3).
fn mod_pm(m: i64, alpha: i64) -> i64 {
    let r = m.rem_euclid(alpha);
    if r > alpha / 2 { r - alpha } else { r }
}

/// Power2Round, FIPS 204 Algorithm 35. Returns (r1, r0).
#[must_use]
pub fn power2round(r: i64) -> (i64, i64) {
    let r_plus = mod_plus(r, Q);
    let r0 = mod_pm(r_plus, 1 << D);
    ((r_plus - r0) >> D, r0)
}

/// Decompose, FIPS 204 Algorithm 36. Returns (r1, r0).
#[must_use]
pub fn decompose(r: i64, gamma2: i64) -> (i64, i64) {
    let r_plus = mod_plus(r, Q);
    let mut r0 = mod_pm(r_plus, 2 * gamma2);
    let r1;
    if r_plus - r0 == Q - 1 {
        r1 = 0;
        r0 -= 1;
    } else {
        r1 = (r_plus - r0) / (2 * gamma2);
    }
    (r1, r0)
}

/// HighBits, FIPS 204 Algorithm 37.
#[must_use]
pub fn high_bits(r: i64, gamma2: i64) -> i64 {
    decompose(r, gamma2).0
}

/// MakeHint, FIPS 204 Algorithm 39. The sum r + z is taken in Z_q.
#[must_use]
pub fn make_hint(z: i64, r: i64, gamma2: i64) -> bool {
    high_bits(r, gamma2) != high_bits(mod_plus(r + z, Q), gamma2)
}

/// UseHint, FIPS 204 Algorithm 40.
#[must_use]
pub fn use_hint(h: bool, r: i64, gamma2: i64) -> i64 {
    let m = (Q - 1) / (2 * gamma2);
    let (r1, r0) = decompose(r, gamma2);
    if h && r0 > 0 {
        (r1 + 1).rem_euclid(m)
    } else if h {
        (r1 - 1).rem_euclid(m)
    } else {
        r1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Algorithm 35's contract, over the whole domain [0, q):
    /// r1 * 2^d + r0 = r and r0 in (-2^(d-1), 2^(d-1)].
    #[test]
    fn power2round_exhaustive() {
        let half = 1i64 << (D - 1);
        for r in 0..Q {
            let (r1, r0) = power2round(r);
            assert_eq!((r1 << D) + r0, r, "r={r}");
            assert!(-half < r0 && r0 <= half, "r={r} r0={r0}");
            assert!((0..=(Q - 1) >> D).contains(&r1), "r={r} r1={r1}");
        }
    }

    /// Algorithm 36's contract, over the whole domain [0, q):
    /// r1 * 2 gamma2 + r0 = r mod q, r1 in [0, (q-1)/(2 gamma2)), and
    /// -gamma2 <= r0 <= gamma2 (the special case can give r0 = -gamma2).
    #[test]
    fn decompose_exhaustive_ml_dsa_65() {
        let g = GAMMA2_65;
        let m = (Q - 1) / (2 * g);
        for r in 0..Q {
            let (r1, r0) = decompose(r, g);
            assert_eq!((r1 * 2 * g + r0).rem_euclid(Q), r, "r={r}");
            assert!((0..m).contains(&r1), "r={r} r1={r1}");
            assert!(-g <= r0 && r0 <= g, "r={r} r0={r0}");
        }
    }

    #[test]
    fn make_hint_of_zero_is_false() {
        for r in (0..Q).step_by(997) {
            assert!(!make_hint(0, r, GAMMA2_65));
        }
    }

    proptest! {
        /// Lemma 1 of the CRYSTALS-Dilithium specification (reference [6]
        /// of FIPS 204, section 2.4): for |z| <= gamma2,
        /// UseHint(MakeHint(z, r), r) = HighBits(r + z).
        #[test]
        fn use_hint_recovers_high_bits(r in 0..Q, z in -GAMMA2_65..=GAMMA2_65) {
            let h = make_hint(z, r, GAMMA2_65);
            prop_assert_eq!(
                use_hint(h, r, GAMMA2_65),
                high_bits((r + z).rem_euclid(Q), GAMMA2_65)
            );
        }
    }
}
