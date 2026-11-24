## What and why

<!-- What changed, and why. Link the milestone in docs/PLAN.md. -->

## Checks run

<!-- Paste the result lines. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] `cargo nextest run --workspace --locked`
- [ ] `cargo deny --locked check licenses bans sources` and `cargo audit`
- [ ] `otb tasks validate`

## Evidence

- New reference values and their sources (standard section, ACVP file, paper table, or command):
- Change in oracle behaviour (`otb selftest` before and after), if any:
- New or changed `KNOWN_GAPS.md` entries:

## Review checklist (reviewer)

- [ ] Consistent with `protocol/APPLICATION.md`; no commercial wording
- [ ] Standards cited correctly
- [ ] Cannot hide a real failure or invent a false one
- [ ] Every number names its command
- [ ] Deterministic output; untrusted code only in the sandbox; no network in tests
