//! Independent, non-LLM checks for oracle-trust-bench.
//!
//! [`speclit`] holds spec-literal Rust transcriptions of the FIPS 203 and
//! FIPS 204 functions used by the pilot tasks. They are one of the two
//! reference backends of the `reference-intermediate` oracle; the other is the
//! instrumented C reference (mlkem-native, mldsa-native). A reference value is
//! trusted only when both agree (GAP-REF-001).

pub mod speclit;
