# Notices

## This repository

- Code (`crates/`, `c/`, `analysis/`, `.github/`): MIT, see `LICENSE`.
- Task specifications, protocol documents and data (`tasks/`, `protocol/`,
  `data/`, `docs/`): CC BY 4.0, see `LICENSE-DATA`. `LICENSE-DATA` is the
  legal code downloaded from https://creativecommons.org/licenses/by/4.0/legalcode.txt
  (sha256 `9ba9550ad48438d0836ddab3da480b3b69ffa0aac7b7878b5a0039e7ab429411`
  after line endings were normalized to LF).

## Third-party material

Each entry gives the source URL, the pinned revision and the licence. Entries
are added when the material is added (milestone M3 and later). Nothing
third-party is vendored yet.

| Material | Source | Revision | Licence | Used for |
|---|---|---|---|---|
| (none yet) | | | | |

## Standards cited

FIPS 203 and FIPS 204 are cited by section, algorithm and equation number.
Their text is not included in this repository. See
`docs/notes/fips-verification.md` for the exact PDFs read and their hashes.

## Rust dependencies

Rust crates are fetched from crates.io at the versions pinned in
`Cargo.lock`. Their licences are checked in CI with `cargo deny` against the
allow-list in `deny.toml`.
