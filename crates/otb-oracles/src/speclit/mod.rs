//! Spec-literal implementations: straight transcriptions of the FIPS
//! pseudocode, written for clarity, not speed and not constant time. They
//! are never given to candidates and never run on secret data.
//!
//! Each function names the algorithm or equation it transcribes. The tests
//! check properties that the standards state, over the whole input domain
//! where it is small enough.

pub mod mldsa;
pub mod mlkem;
