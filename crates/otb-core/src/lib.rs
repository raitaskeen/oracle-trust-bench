//! Core types for oracle-trust-bench.
//!
//! This crate holds the data model shared by every other crate:
//!
//! - stable identifiers ([`TaskId`], [`CandidateId`]);
//! - the closed set of independent checks ([`OracleKind`]);
//! - per-oracle results ([`Verdict`], [`Outcome`], [`Evidence`]);
//! - canonical JSON output ([`canonical`]), so that the same inputs always
//!   produce byte-identical files.
//!
//! The crate performs no I/O and runs no code.

pub mod canonical;
mod ids;
mod oracle;
mod verdict;

pub use ids::{CandidateId, IdError, TaskId, ToolchainId};
pub use oracle::OracleKind;
pub use verdict::{Evidence, FailureClass, Outcome, Verdict};
