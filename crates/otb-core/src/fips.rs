//! Algorithm numbers and names of FIPS 203 and FIPS 204.
//!
//! Transcribed from the "List of Algorithms" of each standard (the PDFs whose
//! hashes are recorded in `docs/notes/fips-verification.md`). Task specs cite
//! algorithms by number and name, and validation checks both against these
//! tables, so a mistyped number cannot pass unnoticed.

use serde::{Deserialize, Serialize};

/// A FIPS document cited by the pilot tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FipsDoc {
    /// FIPS 203, Module-Lattice-Based Key-Encapsulation Mechanism Standard.
    #[serde(rename = "FIPS 203")]
    Fips203,
    /// FIPS 204, Module-Lattice-Based Digital Signature Standard.
    #[serde(rename = "FIPS 204")]
    Fips204,
}

impl FipsDoc {
    /// The document's name, e.g. `FIPS 203`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Fips203 => "FIPS 203",
            Self::Fips204 => "FIPS 204",
        }
    }

    /// The name of algorithm `n`, as printed in the standard's list of
    /// algorithms (subscripts and Greek letters written in ASCII).
    #[must_use]
    pub fn algorithm_name(self, n: u32) -> Option<&'static str> {
        let table: &[&str] = match self {
            Self::Fips203 => FIPS203_ALGORITHMS,
            Self::Fips204 => FIPS204_ALGORITHMS,
        };
        let idx = usize::try_from(n).ok()?.checked_sub(1)?;
        table.get(idx).copied()
    }
}

/// FIPS 203 Algorithms 1 to 21.
const FIPS203_ALGORITHMS: &[&str] = &[
    "ForExample",
    "SHAKE128example",
    "BitsToBytes",
    "BytesToBits",
    "ByteEncode_d",
    "ByteDecode_d",
    "SampleNTT",
    "SamplePolyCBD_eta",
    "NTT",
    "NTT^-1",
    "MultiplyNTTs",
    "BaseCaseMultiply",
    "K-PKE.KeyGen",
    "K-PKE.Encrypt",
    "K-PKE.Decrypt",
    "ML-KEM.KeyGen_internal",
    "ML-KEM.Encaps_internal",
    "ML-KEM.Decaps_internal",
    "ML-KEM.KeyGen",
    "ML-KEM.Encaps",
    "ML-KEM.Decaps",
];

/// FIPS 204 Algorithms 1 to 49.
const FIPS204_ALGORITHMS: &[&str] = &[
    "ML-DSA.KeyGen",
    "ML-DSA.Sign",
    "ML-DSA.Verify",
    "HashML-DSA.Sign",
    "HashML-DSA.Verify",
    "ML-DSA.KeyGen_internal",
    "ML-DSA.Sign_internal",
    "ML-DSA.Verify_internal",
    "IntegerToBits",
    "BitsToInteger",
    "IntegerToBytes",
    "BitsToBytes",
    "BytesToBits",
    "CoeffFromThreeBytes",
    "CoeffFromHalfByte",
    "SimpleBitPack",
    "BitPack",
    "SimpleBitUnpack",
    "BitUnpack",
    "HintBitPack",
    "HintBitUnpack",
    "pkEncode",
    "pkDecode",
    "skEncode",
    "skDecode",
    "sigEncode",
    "sigDecode",
    "w1Encode",
    "SampleInBall",
    "RejNTTPoly",
    "RejBoundedPoly",
    "ExpandA",
    "ExpandS",
    "ExpandMask",
    "Power2Round",
    "Decompose",
    "HighBits",
    "LowBits",
    "MakeHint",
    "UseHint",
    "NTT",
    "NTT^-1",
    "BitRev8",
    "AddNTT",
    "MultiplyNTT",
    "AddVectorNTT",
    "ScalarVectorNTT",
    "MatrixVectorNTT",
    "MontgomeryReduce",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_sizes_match_the_standards() {
        assert_eq!(FIPS203_ALGORITHMS.len(), 21);
        assert_eq!(FIPS204_ALGORITHMS.len(), 49);
    }

    #[test]
    fn spot_checks() {
        assert_eq!(FipsDoc::Fips203.algorithm_name(9), Some("NTT"));
        assert_eq!(
            FipsDoc::Fips203.algorithm_name(18),
            Some("ML-KEM.Decaps_internal")
        );
        assert_eq!(FipsDoc::Fips203.algorithm_name(21), Some("ML-KEM.Decaps"));
        assert_eq!(FipsDoc::Fips204.algorithm_name(35), Some("Power2Round"));
        assert_eq!(FipsDoc::Fips204.algorithm_name(39), Some("MakeHint"));
        assert_eq!(FipsDoc::Fips203.algorithm_name(0), None);
        assert_eq!(FipsDoc::Fips203.algorithm_name(22), None);
        assert_eq!(FipsDoc::Fips204.algorithm_name(50), None);
    }
}
