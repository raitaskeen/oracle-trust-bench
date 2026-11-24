# Notices

## This repository

- Code (`crates/`, `c/`, `analysis/`, `.github/`): MIT, see `LICENSE`.
- Task specifications, protocol documents and data (`tasks/`, `protocol/`,
  `data/`, `docs/`): CC BY 4.0, see `LICENSE-DATA`. `LICENSE-DATA` is the
  legal code downloaded from https://creativecommons.org/licenses/by/4.0/legalcode.txt
  (sha256 `9ba9550ad48438d0836ddab3da480b3b69ffa0aac7b7878b5a0039e7ab429411`
  after line endings were normalized to LF).

## Third-party material

Each entry gives the source URL, the pinned revision and the licence.

| Material | Source | Revision | Licence | Used for |
|---|---|---|---|---|
| ACVP sample vectors: `ML-KEM-keyGen-FIPS203`, `ML-KEM-encapDecap-FIPS203`, `ML-DSA-keyGen-FIPS204`, `ML-DSA-sigGen-FIPS204`, `ML-DSA-sigVer-FIPS204` (`internalProjection.json` only) | https://github.com/usnistgov/ACVP-Server, `gen-val/json-files/` | commit `975de31eb83d87039ec88934fdc47d8c312b892d` | NIST public-software notice: NIST-developed software is not subject to copyright protection within the United States (see the repository README). No SPDX identifier. | `acvp` oracle; validation of the spec-literal reference |

The vendored files are byte-identical to the source; their SHA-256 hashes
are in `third_party/acvp/SHA256SUMS`. The files' `isSample` flag is `true` for
`ML-KEM-encapDecap-FIPS203` and `false` for the other four. All five are
example vector sets published in the ACVP-Server repository, not vectors
from a validation session run for this study (GAP-ACVP-002).

## Standards cited

FIPS 203 and FIPS 204 are cited by section, algorithm and equation number.
Their text is not included in this repository. See
`docs/notes/fips-verification.md` for the exact PDFs read and their hashes.

## Rust dependencies

Rust crates are fetched from crates.io at the versions pinned in
`Cargo.lock`. Their licences are checked in CI with `cargo deny` against the
allow-list in `deny.toml`.
