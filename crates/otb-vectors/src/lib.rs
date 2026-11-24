//! Strict parsers for NIST ACVP test vector files (`internalProjection.json`
//! from the ACVP-Server repository, which holds inputs and expected outputs
//! together).
//!
//! Parsing is strict: unknown fields are errors, so a change in the vector
//! format fails loudly instead of silently skipping data. The vectors are
//! vendored under `third_party/acvp/` at a pinned commit (see `NOTICE.md`).

pub mod hex;
pub mod mldsa;
pub mod mlkem;

use std::path::Path;

/// Errors from reading a vector file.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// Reading the file failed.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: String,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The file is not the expected JSON shape.
    #[error("{path}: {source}")]
    Json {
        /// The file.
        path: String,
        /// The parse error.
        source: serde_json::Error,
    },
}

/// Reads and parses a JSON vector file.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, VectorError> {
    let text = std::fs::read_to_string(path).map_err(|source| VectorError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| VectorError::Json {
        path: path.display().to_string(),
        source,
    })
}
