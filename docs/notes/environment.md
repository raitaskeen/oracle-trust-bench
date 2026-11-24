# Development environment (M0)

Recorded 24 Nov 2026 on the author's machine. Versions were printed by the
commands shown.

| Component | Version | Command |
|---|---|---|
| Host | Windows 11 Pro, WSL2 | `wsl --status` |
| Linux | Ubuntu 24.04 LTS | `cat /etc/os-release` |
| Rust | rustc 1.100.0 (0ef1bc386 2026-11-09) | `rustc --version` |
| GCC (host) | 13.3.0 (Ubuntu 13.3.0-6ubuntu2~24.04.1) | `gcc --version` |
| Clang (host) | Ubuntu clang 18.1.3 | `clang --version` |
| Valgrind (host) | 3.22.0 | `valgrind --version` |
| Python / SciPy | python3 with SciPy 1.11.4 | `python3 -c 'import scipy; print(scipy.__version__)'` |
| Docker Engine | 29.1.3 (cgroup v2, overlayfs) | `docker info` |
| cargo-nextest | 0.9.152 | `cargo nextest --version` |
| cargo-deny | 0.22.0 | `cargo deny --version` |
| cargo-audit | 0.23.0 | `cargo audit --version` |
| Base image | `debian:trixie-slim`, Debian 13.8, `sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0` | `docker image inspect` |

The host compilers above are not the study's compilers. Candidate and
reference C is compiled only inside the pinned image (ADR-001, ADR-005).

Unprivileged network namespaces work on this host (`unshare -r -n true`),
so the weaker fallback sandbox is available.

Checks run here on 24 Nov 2026 at commit `c88d45f`: fmt, clippy
(`-D warnings`), 55/55 tests (also with the network removed),
`cargo deny` and `cargo audit` all pass; `otb budget` and
`python3 analysis/budget.py` both print `TOTAL 955.98`.

## Candidate image (M3)

Built on 24 Nov 2026 from `docker/Dockerfile` with
`docker build -t otb-candidate:dev docker/` (packages from
deb.debian.org, exact versions pinned in the Dockerfile).

| Item | Value | Command |
|---|---|---|
| Image ID | `sha256:d07dce05bd48c4d3b894505ff5f9bde94e06f058574e5ca33046fa64aad77ed9` | `docker image inspect otb-candidate:dev --format '{{.Id}}'` |
| GCC | gcc (Debian 14.2.0-19) 14.2.0 | `docker run --rm otb-candidate:dev gcc --version` |
| Clang | Debian clang version 19.1.7 (3+b1) | `docker run --rm otb-candidate:dev clang --version` |
| Valgrind | valgrind-3.24.0 | `docker run --rm otb-candidate:dev valgrind --version` |
| User | uid=10001(otb) gid=10001(otb) | `docker run --rm otb-candidate:dev id` |

The image ID is local to this build: a rebuild of the same Dockerfile
installs the same package versions but gets a new ID (layer timestamps
differ). CI builds its own image and passes its ID through
`OTB_SANDBOX_IMAGE`. Fixing one image by content digest for the main run
is still open (GAP-ENV-002).

With `OTB_SANDBOX_IMAGE` set to the ID above:

- `cargo nextest run --locked -p otb-sandbox --run-ignored all`: 9 tests
  run, 9 passed, including `docker_blocks_connect`,
  `unshare_fallback_blocks_connect`, the wall-clock kill on both backends
  and the read-only root and input mount under Docker.
- `otb ref run` (Docker backend): every check passes; T41 8380417 cases,
  T47 1000340, T01 1258, T07 16645, T24 ACVP 20, ML-DSA-65 ACVP 160
  (45 pre-hash cases not covered, GAP-ACVP-003); 16 s wall-clock.
