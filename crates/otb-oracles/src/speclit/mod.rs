//! Spec-literal implementations: straight transcriptions of the FIPS
//! pseudocode, written for clarity, not speed and not constant time. They
//! are never given to candidates and never run on secret data.
//!
//! Each function names the algorithm or equation it transcribes. The tests
//! check properties that the standards state, over the whole input domain
//! where it is small enough.
//!
//! Variable names follow the standards (k, m, c, r, z, ...) so the code can be
//! read line by line against the pseudocode. Every function panics on inputs
//! outside the domain the standard defines; that is documented per module
//! rather than per function.
#![allow(clippy::many_single_char_names, clippy::missing_panics_doc)]

pub mod mldsa;
pub mod mlkem;
pub mod mlkem_kem;
