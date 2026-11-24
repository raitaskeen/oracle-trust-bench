# Contributing

This repository is the public home of a pre-registered research study.
Changes are welcome, but they are reviewed for scientific correctness as
well as code quality. AI agents follow the same rules; see `AGENTS.md`.

## Before you open a pull request

Run the checks in `docs/TESTING.md` and make sure they all pass. A pull
request that fails CI is not reviewed.

## What every pull request must say

The template in `.github/pull_request_template.md` asks for:

- what changed and why;
- which milestone in `docs/PLAN.md` it advances;
- the commands you ran and their results;
- any new reference value, with its source;
- any change to oracle behaviour, with the before and after of
  `otb selftest` (from M6);
- any new or changed entry in `KNOWN_GAPS.md`.

## How a pull request is reviewed

A review is a recorded event, not an assumption. If a bot or an agent was
asked to review and did not, the pull request counts as unreviewed.

The reviewer checks, in this order:

1. **Protocol.** Does the change contradict `protocol/APPLICATION.md` or the
   pre-registration? Does it add product or commercial wording?
2. **Standards.** Are FIPS 203, FIPS 204 and ACVP cited correctly? Does a
   new rule conflict with a standard?
3. **Oracle soundness.** Could the change hide a real failure (a weaker
   check, a changed expected value, a widened declassification) or invent
   one (a false positive on the references)?
4. **Evidence.** Does every new number name its command, and every new
   reference value its source?
5. **Determinism and safety.** No timestamps or paths in hashed output; no
   untrusted code outside the sandbox; no network in tests.
6. **Code.** Readable, tested, no abstraction without two real uses.

Changes to `crates/otb-oracles`, `crates/otb-sandbox`, `tasks/`, `protocol/`
or `.github/` need a second reviewer once the project has one.

## Commits

Small commits with a clear message saying what changed and why. AI-assisted
commits carry a `Co-Authored-By` line naming the model.

## Licences

Code is MIT (`LICENSE`). Specs, protocol, data and documentation are
CC BY 4.0 (`LICENSE-DATA`). By contributing you agree to these terms.
