# Instructions for AI coding agents

This file is for any AI agent (Claude Code, Codex, others) working in this
repository. Human contributors should read `CONTRIBUTING.md`. Both follow the
same rules.

## What this repository is

The public home of a pre-registered research study, "The Oracle-Trust Gap".
It holds task specs, independent non-LLM oracles, a sandboxed harness that
checks C implementations, and the protocol. The primary record of what the
study promises is `protocol/APPLICATION.md`. Nothing here may contradict it.

It is research, not a product. Never add product, platform, customer,
pricing, market or startup wording.

## Hard rules

1. **No invented facts.** No invented numbers, results, citations or badges.
   Every number in a document names the command that produced it. A test
   reference value names its source (paper, standard, or command).
2. **No results yet.** Until the study has run, documents say "No model has
   been run. No results yet."
3. **Standards win.** When an instruction conflicts with FIPS 203, FIPS 204
   or ACVP, follow the standard and record the conflict in
   `docs/notes/fips-verification.md`.
4. **Never weaken a check to make it pass.** If an oracle disagrees with a
   reference, investigate and record the finding in `KNOWN_GAPS.md`. Never
   edit expected values to match.
5. **Candidate C code is untrusted.** Compile and run it only through
   `otb-sandbox`. Never include or link it into a Rust process. Never mount
   expected values into a candidate's sandbox.
6. **Determinism.** Verdict files must be byte-identical across runs and
   `--jobs` values. Use `BTreeMap`, sort outputs by stable keys, and keep
   timestamps, paths, PIDs and raw tool logs out of hashed files.
7. **No network, no secrets in tests.** Vectors and reference code are
   vendored or pinned submodules, recorded in `NOTICE.md`.
8. **Pin everything.** Toolchain in `rust-toolchain.toml`, crates in
   `Cargo.lock`, container images by digest, submodules by commit, GitHub
   Actions by full commit SHA.
9. **Small scope.** Build only what the pilot needs. An abstraction needs two
   real uses. No plugin system, web UI, database or dashboard.
10. **Git.** Small commits with clear messages. Never push, add a remote,
    tag a release or open a pull request: the author does that.
11. **Known gaps.** Cite limits by their ID in `KNOWN_GAPS.md`. The test
    `crates/otb-core/tests/known_gaps.rs` fails on an undefined ID.

## Checks to run before every commit

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked
```

## Layout

- `crates/otb-core`: IDs, oracle kinds, verdicts, canonical JSON.
- `tasks/`: one directory per task, with its `spec.md`.
- `protocol/`: application text, pre-registration draft, oracle matrix.
- `docs/notes/`: working notes, including FIPS verification.
- `docs/adr/`: one-page architecture decisions.
- `KNOWN_GAPS.md`: the ledger of known limits.
