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

Checks run here on 24 Nov 2026 at commit `d248392`: fmt, clippy
(`-D warnings`), 55/55 tests (also with the network removed),
`cargo deny` and `cargo audit` all pass; `otb budget` and
`python3 analysis/budget.py` both print `TOTAL 955.98`.
