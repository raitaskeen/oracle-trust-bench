# ADR-002: Canonical JSON and stable IDs

Status: accepted (M1, 24 Nov 2026)

## Context

The study's claims rest on verdict files. A reader must be able to rerun the
harness and get the same bytes, and to diff two runs. That requires stable
identifiers and one serialization for every hashed file.

## Decision

**Canonical JSON.** Every hashed file is written by `otb_core::canonical`,
which implements RFC 8785 (JCS) for documents without floating-point numbers:

- no insignificant whitespace;
- object members sorted by name as UTF-16 code units, recursively;
- strings escaped as RFC 8785 section 3.2.2.2 requires;
- UTF-8 output;
- **any non-integer number is an error.** Rates are stored as integer counts
  (numerator and denominator) and money as integer micro-dollars. This keeps
  the ECMAScript number-formatting rules of RFC 8785 out of the code.

Tests use the RFC's sorting example (section 3.2.3) and its string and
literals example (section 3.2.2, with the float member removed).

**Task IDs.** Format `<MLKEM|MLDSA>-T<NN>-<slug>`, for example
`MLKEM-T01-ntt`. Validated on parse. The two-digit number matches the
application's task numbering.

**Candidate IDs.** The SHA-256 of the normalized source, written as 64
lowercase hex digits. Normalization is minimal: the source must be UTF-8,
CRLF becomes LF, and trailing LFs are replaced by exactly one. Nothing else
changes, so two programs that differ in any meaningful way never share an ID.

**No timestamps in verdicts.** A `Verdict` holds task, candidate, oracle,
outcome and toolchain only. Run times, host details and raw tool logs go to
an unhashed run log.

## Consequences

- Byte-identical output can be checked with `cmp`, at any `--jobs` value.
- A float can never enter a hashed file by accident: serialization fails.
- Changing normalization or ID format is a breaking change to every stored
  verdict, so it needs a new ADR.
