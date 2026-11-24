//! FIPS 203 K-PKE and ML-KEM, transcribed from the pseudocode
//! (Algorithms 3-8, 10-18 and the section 7 input checks).
//!
//! Written for clarity: polynomials are `[u32; 256]` with every coefficient
//! kept in [0, q). Validated against every official ACVP ML-KEM keyGen,
//! encapsulation and decapsulation sample vector (see `tests/acvp_mlkem.rs`).

use sha3::{Digest, Sha3_256, Sha3_512};
use shake::{ExtendableOutput, Shake128, Shake256, Update, XofReader};

use super::mlkem::{Q, ZETA, bitrev7, compress, decompress, ntt};

/// One polynomial in R_q or T_q, coefficients in [0, q).
pub type Poly = [u32; 256];

/// An ML-KEM parameter set (FIPS 203 section 8, Table 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// Module rank.
    pub k: usize,
    /// CBD parameter for s, e and y.
    pub eta1: usize,
    /// CBD parameter for e1 and e2.
    pub eta2: usize,
    /// Compression width for u.
    pub du: u32,
    /// Compression width for v.
    pub dv: u32,
}

/// ML-KEM-512.
pub const ML_KEM_512: Params = Params {
    k: 2,
    eta1: 3,
    eta2: 2,
    du: 10,
    dv: 4,
};
/// ML-KEM-768.
pub const ML_KEM_768: Params = Params {
    k: 3,
    eta1: 2,
    eta2: 2,
    du: 10,
    dv: 4,
};
/// ML-KEM-1024.
pub const ML_KEM_1024: Params = Params {
    k: 4,
    eta1: 2,
    eta2: 2,
    du: 11,
    dv: 5,
};

impl Params {
    /// Encapsulation key length, 384k + 32 (Table 3).
    #[must_use]
    pub fn ek_len(self) -> usize {
        384 * self.k + 32
    }
    /// Decapsulation key length, 768k + 96 (Table 3).
    #[must_use]
    pub fn dk_len(self) -> usize {
        768 * self.k + 96
    }
    /// Ciphertext length, 32(du k + dv) (Table 3).
    #[must_use]
    pub fn ct_len(self) -> usize {
        32 * (self.du as usize * self.k + self.dv as usize)
    }
}

// ---- Hash functions (FIPS 203 section 4.1) ----

/// H(s) = SHA3-256(s), Eq. (4.4).
fn h(s: &[u8]) -> [u8; 32] {
    Sha3_256::digest(s).into()
}

/// J(s) = SHAKE256(s, 8 * 32), Eq. (4.4).
#[must_use]
pub fn j(s: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    shake256(s, &mut out);
    out
}

/// G(c) = SHA3-512(c), split into two 32-byte halves, Eq. (4.5).
fn g(c: &[u8]) -> ([u8; 32], [u8; 32]) {
    let d: [u8; 64] = Sha3_512::digest(c).into();
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    a.copy_from_slice(&d[..32]);
    b.copy_from_slice(&d[32..]);
    (a, b)
}

/// PRF_eta(s, b) = SHAKE256(s || b, 8 * 64 * eta), Eq. (4.3).
fn prf(eta: usize, s: &[u8; 32], b: u8) -> Vec<u8> {
    let mut input = s.to_vec();
    input.push(b);
    let mut out = vec![0u8; 64 * eta];
    shake256(&input, &mut out);
    out
}

fn shake256(input: &[u8], out: &mut [u8]) {
    let mut x = Shake256::default();
    x.update(input);
    x.finalize_xof().read(out);
}

// ---- Bits, bytes and encoding (Algorithms 3-6) ----

/// BytesToBits, Algorithm 4: little-endian bit order within each byte.
fn bytes_to_bits(b: &[u8]) -> Vec<u8> {
    b.iter()
        .flat_map(|&byte| (0..8).map(move |i| (byte >> i) & 1))
        .collect()
}

/// BitsToBytes, Algorithm 3.
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    bits.chunks(8)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u8, |acc, (i, &b)| acc | (b << i))
        })
        .collect()
}

/// ByteEncode_d, Algorithm 5.
#[must_use]
pub fn byte_encode(d: u32, f: &Poly) -> Vec<u8> {
    let mut bits = Vec::with_capacity(256 * d as usize);
    for &a in f {
        for i in 0..d {
            bits.push(u8::try_from((a >> i) & 1).expect("one bit"));
        }
    }
    bits_to_bytes(&bits)
}

/// ByteDecode_d, Algorithm 6. For d = 12 the result is reduced mod q.
#[must_use]
pub fn byte_decode(d: u32, b: &[u8]) -> Poly {
    assert_eq!(b.len(), 32 * d as usize, "ByteDecode_d needs 32d bytes");
    let bits = bytes_to_bits(b);
    let m = if d < 12 { 1u32 << d } else { Q };
    let mut f = [0u32; 256];
    for (i, c) in f.iter_mut().enumerate() {
        let v: u32 = (0..d as usize)
            .map(|jj| u32::from(bits[i * d as usize + jj]) << jj)
            .sum();
        *c = v % m;
    }
    f
}

// ---- Sampling (Algorithms 7, 8) ----

/// SampleNTT, Algorithm 7, on the 34-byte input rho || j || i.
fn sample_ntt(seed: &[u8; 34]) -> Poly {
    let mut x = Shake128::default();
    x.update(seed);
    let mut reader = x.finalize_xof();
    let mut a = [0u32; 256];
    let mut jj = 0;
    while jj < 256 {
        let mut c = [0u8; 3];
        reader.read(&mut c);
        let d1 = u32::from(c[0]) + 256 * (u32::from(c[1]) % 16);
        let d2 = u32::from(c[1]) / 16 + 16 * u32::from(c[2]);
        if d1 < Q {
            a[jj] = d1;
            jj += 1;
        }
        if d2 < Q && jj < 256 {
            a[jj] = d2;
            jj += 1;
        }
    }
    a
}

/// SamplePolyCBD_eta, Algorithm 8.
fn sample_poly_cbd(eta: usize, b: &[u8]) -> Poly {
    assert_eq!(b.len(), 64 * eta);
    let bits = bytes_to_bits(b);
    let mut f = [0u32; 256];
    for (i, c) in f.iter_mut().enumerate() {
        let x: u32 = (0..eta).map(|jj| u32::from(bits[2 * i * eta + jj])).sum();
        let y: u32 = (0..eta)
            .map(|jj| u32::from(bits[2 * i * eta + eta + jj]))
            .sum();
        *c = (x + Q - y) % Q;
    }
    f
}

// ---- NTT arithmetic (Algorithms 10-12) ----

fn pow_mod(base: u32, mut e: u32) -> u32 {
    let mut r: u64 = 1;
    let mut b = u64::from(base);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % u64::from(Q);
        }
        b = b * b % u64::from(Q);
        e >>= 1;
    }
    u32::try_from(r).expect("below q")
}

fn mul(a: u32, b: u32) -> u32 {
    u32::try_from(u64::from(a) * u64::from(b) % u64::from(Q)).expect("below q")
}

fn add(a: u32, b: u32) -> u32 {
    (a + b) % Q
}

fn sub(a: u32, b: u32) -> u32 {
    (a + Q - b) % Q
}

fn ntt_poly(f: &Poly) -> Poly {
    let input: [u16; 256] = f.map(|c| u16::try_from(c).expect("below q"));
    ntt(&input).map(u32::from)
}

/// NTT^-1, Algorithm 10.
#[must_use]
pub fn ntt_inv(fh: &Poly) -> Poly {
    let mut f = *fh;
    let mut i: u8 = 127;
    let mut len = 2;
    while len <= 128 {
        let mut start = 0;
        while start < 256 {
            let zeta = pow_mod(ZETA, u32::from(bitrev7(i)));
            i = i.wrapping_sub(1);
            for jj in start..start + len {
                let t = f[jj];
                f[jj] = add(t, f[jj + len]);
                f[jj + len] = mul(zeta, sub(f[jj + len], t));
            }
            start += 2 * len;
        }
        len *= 2;
    }
    f.map(|c| mul(c, 3303))
}

/// MultiplyNTTs, Algorithm 11, with BaseCaseMultiply, Algorithm 12.
fn multiply_ntts(a: &Poly, b: &Poly) -> Poly {
    let mut c = [0u32; 256];
    for i in 0..128 {
        let gamma = pow_mod(
            ZETA,
            2 * u32::from(bitrev7(u8::try_from(i).expect("i < 128"))) + 1,
        );
        let (a0, a1, b0, b1) = (a[2 * i], a[2 * i + 1], b[2 * i], b[2 * i + 1]);
        c[2 * i] = add(mul(a0, b0), mul(mul(a1, b1), gamma));
        c[2 * i + 1] = add(mul(a0, b1), mul(a1, b0));
    }
    c
}

fn poly_add(a: &Poly, b: &Poly) -> Poly {
    let mut c = [0u32; 256];
    for i in 0..256 {
        c[i] = add(a[i], b[i]);
    }
    c
}

fn poly_sub(a: &Poly, b: &Poly) -> Poly {
    let mut c = [0u32; 256];
    for i in 0..256 {
        c[i] = sub(a[i], b[i]);
    }
    c
}

/// Regenerates the matrix A-hat, with A-hat[i][j] = SampleNTT(rho || j || i).
fn gen_matrix(p: Params, rho: &[u8; 32]) -> Vec<Vec<Poly>> {
    (0..p.k)
        .map(|i| {
            (0..p.k)
                .map(|jj| {
                    let mut seed = [0u8; 34];
                    seed[..32].copy_from_slice(rho);
                    seed[32] = u8::try_from(jj).expect("j < k");
                    seed[33] = u8::try_from(i).expect("i < k");
                    sample_ntt(&seed)
                })
                .collect()
        })
        .collect()
}

// ---- K-PKE (Algorithms 13-15) ----

/// K-PKE.KeyGen, Algorithm 13. Returns (ek_PKE, dk_PKE).
#[must_use]
pub fn kpke_keygen(p: Params, d: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let mut seed = d.to_vec();
    seed.push(u8::try_from(p.k).expect("k fits in a byte"));
    let (rho, sigma) = g(&seed);
    let mut n: u8 = 0;
    let a_hat = gen_matrix(p, &rho);
    let mut s = Vec::new();
    for _ in 0..p.k {
        s.push(sample_poly_cbd(p.eta1, &prf(p.eta1, &sigma, n)));
        n += 1;
    }
    let mut e = Vec::new();
    for _ in 0..p.k {
        e.push(sample_poly_cbd(p.eta1, &prf(p.eta1, &sigma, n)));
        n += 1;
    }
    let s_hat: Vec<Poly> = s.iter().map(ntt_poly).collect();
    let e_hat: Vec<Poly> = e.iter().map(ntt_poly).collect();
    let mut ek = Vec::new();
    for i in 0..p.k {
        let mut t = e_hat[i];
        for jj in 0..p.k {
            t = poly_add(&t, &multiply_ntts(&a_hat[i][jj], &s_hat[jj]));
        }
        ek.extend(byte_encode(12, &t));
    }
    ek.extend_from_slice(&rho);
    let dk: Vec<u8> = s_hat.iter().flat_map(|sh| byte_encode(12, sh)).collect();
    (ek, dk)
}

/// K-PKE.Encrypt, Algorithm 14.
#[must_use]
pub fn kpke_encrypt(p: Params, ek: &[u8], m: &[u8; 32], r: &[u8; 32]) -> Vec<u8> {
    let mut n: u8 = 0;
    let t_hat: Vec<Poly> = (0..p.k)
        .map(|i| byte_decode(12, &ek[384 * i..384 * (i + 1)]))
        .collect();
    let mut rho = [0u8; 32];
    rho.copy_from_slice(&ek[384 * p.k..384 * p.k + 32]);
    let a_hat = gen_matrix(p, &rho);
    let mut y = Vec::new();
    for _ in 0..p.k {
        y.push(sample_poly_cbd(p.eta1, &prf(p.eta1, r, n)));
        n += 1;
    }
    let mut e1 = Vec::new();
    for _ in 0..p.k {
        e1.push(sample_poly_cbd(p.eta2, &prf(p.eta2, r, n)));
        n += 1;
    }
    let e2 = sample_poly_cbd(p.eta2, &prf(p.eta2, r, n));
    let y_hat: Vec<Poly> = y.iter().map(ntt_poly).collect();

    let mut c = Vec::new();
    for i in 0..p.k {
        // u[i] = NTT^-1(sum_j A-hat[j][i] * y-hat[j]) + e1[i]  (A-hat transposed)
        let mut acc = [0u32; 256];
        for jj in 0..p.k {
            acc = poly_add(&acc, &multiply_ntts(&a_hat[jj][i], &y_hat[jj]));
        }
        let u = poly_add(&ntt_inv(&acc), &e1[i]);
        c.extend(byte_encode(p.du, &u.map(|x| compress(p.du, x))));
    }
    let mu = byte_decode(1, m).map(|b| decompress(1, b));
    let mut acc = [0u32; 256];
    for jj in 0..p.k {
        acc = poly_add(&acc, &multiply_ntts(&t_hat[jj], &y_hat[jj]));
    }
    let v = poly_add(&poly_add(&ntt_inv(&acc), &e2), &mu);
    c.extend(byte_encode(p.dv, &v.map(|x| compress(p.dv, x))));
    c
}

/// K-PKE.Decrypt, Algorithm 15.
#[must_use]
pub fn kpke_decrypt(p: Params, dk: &[u8], c: &[u8]) -> [u8; 32] {
    let du = p.du as usize;
    let c1 = &c[..32 * du * p.k];
    let c2 = &c[32 * du * p.k..];
    let mut acc = [0u32; 256];
    for i in 0..p.k {
        let u = byte_decode(p.du, &c1[32 * du * i..32 * du * (i + 1)]).map(|x| decompress(p.du, x));
        let s_hat = byte_decode(12, &dk[384 * i..384 * (i + 1)]);
        acc = poly_add(&acc, &multiply_ntts(&s_hat, &ntt_poly(&u)));
    }
    let v = byte_decode(p.dv, c2).map(|x| decompress(p.dv, x));
    let w = poly_sub(&v, &ntt_inv(&acc));
    let m = byte_encode(1, &w.map(|x| compress(1, x)));
    m.try_into().expect("32 bytes")
}

// ---- ML-KEM internal algorithms (Algorithms 16-18) ----

/// ML-KEM.KeyGen_internal, Algorithm 16. Returns (ek, dk).
#[must_use]
pub fn keygen_internal(p: Params, d: &[u8; 32], z: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let (ek, dk_pke) = kpke_keygen(p, d);
    let mut dk = dk_pke;
    dk.extend_from_slice(&ek);
    dk.extend_from_slice(&h(&ek));
    dk.extend_from_slice(z);
    (ek, dk)
}

/// ML-KEM.Encaps_internal, Algorithm 17. Returns (K, c).
#[must_use]
pub fn encaps_internal(p: Params, ek: &[u8], m: &[u8; 32]) -> ([u8; 32], Vec<u8>) {
    let mut input = m.to_vec();
    input.extend_from_slice(&h(ek));
    let (k, r) = g(&input);
    (k, kpke_encrypt(p, ek, m, &r))
}

/// ML-KEM.Decaps_internal, Algorithm 18.
#[must_use]
pub fn decaps_internal(p: Params, dk: &[u8], c: &[u8]) -> [u8; 32] {
    let k = p.k;
    let dk_pke = &dk[..384 * k];
    let ek_pke = &dk[384 * k..768 * k + 32];
    let hh = &dk[768 * k + 32..768 * k + 64];
    let z = &dk[768 * k + 64..768 * k + 96];
    let m_prime = kpke_decrypt(p, dk_pke, c);
    let mut input = m_prime.to_vec();
    input.extend_from_slice(hh);
    let (k_prime, r_prime) = g(&input);
    let mut zc = z.to_vec();
    zc.extend_from_slice(c);
    let k_bar = j(&zc);
    let c_prime = kpke_encrypt(p, ek_pke, &m_prime, &r_prime);
    if c == c_prime.as_slice() {
        k_prime
    } else {
        k_bar
    }
}

// ---- Input checks (FIPS 203 sections 7.2 and 7.3) ----

/// Encapsulation key check, section 7.2: type check and modulus check (Eq. 7.1).
#[must_use]
pub fn ek_check(p: Params, ek: &[u8]) -> bool {
    if ek.len() != p.ek_len() {
        return false;
    }
    (0..p.k).all(|i| {
        let chunk = &ek[384 * i..384 * (i + 1)];
        byte_encode(12, &byte_decode(12, chunk)) == chunk
    })
}

/// Decapsulation input check, section 7.3: ciphertext type check,
/// decapsulation key type check, and hash check (Eq. 7.2).
#[must_use]
pub fn decaps_input_check(p: Params, dk: &[u8], c: &[u8]) -> bool {
    c.len() == p.ct_len() && dk_check(p, dk)
}

/// The decapsulation-key part of the section 7.3 check: type check and
/// hash check H(dk[384k : 768k+32]) = dk[768k+32 : 768k+64].
#[must_use]
pub fn dk_check(p: Params, dk: &[u8]) -> bool {
    let k = p.k;
    dk.len() == p.dk_len() && h(&dk[384 * k..768 * k + 32]) == dk[768 * k + 32..768 * k + 64]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_match_table_3() {
        assert_eq!(
            (
                ML_KEM_512.ek_len(),
                ML_KEM_512.dk_len(),
                ML_KEM_512.ct_len()
            ),
            (800, 1632, 768)
        );
        assert_eq!(
            (
                ML_KEM_768.ek_len(),
                ML_KEM_768.dk_len(),
                ML_KEM_768.ct_len()
            ),
            (1184, 2400, 1088)
        );
        assert_eq!(
            (
                ML_KEM_1024.ek_len(),
                ML_KEM_1024.dk_len(),
                ML_KEM_1024.ct_len()
            ),
            (1568, 3168, 1568)
        );
    }

    #[test]
    fn ntt_inv_inverts_ntt_on_unit_vectors() {
        for jj in 0..256 {
            let mut f = [0u32; 256];
            f[jj] = 1;
            assert_eq!(ntt_inv(&ntt_poly(&f)), f, "unit vector {jj}");
        }
    }

    #[test]
    fn byte_encode_decode_round_trip() {
        let mut f = [0u32; 256];
        for (i, c) in f.iter_mut().enumerate() {
            *c = u32::try_from(i * 13 % 3329).unwrap();
        }
        assert_eq!(byte_decode(12, &byte_encode(12, &f)), f);
        let small = f.map(|c| c % 16);
        assert_eq!(byte_decode(4, &byte_encode(4, &small)), small);
    }
}
