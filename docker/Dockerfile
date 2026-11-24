# Image in which all C code (references, drivers, seeded faults and
# candidates) is compiled and run. See ADR-003 (sandbox) and ADR-005
# (constant-time method).
#
# Reproducibility:
# - the base image is pinned by digest (Debian 13.8, trixie-slim);
# - every compiler and tool package is pinned to an exact version. If the
#   Debian mirror no longer carries a version, the build fails instead of
#   silently installing a different compiler;
# - the built image is referenced by its ID in every run record.
#
# Packages come from deb.debian.org (a CDN). An earlier version of this file
# used snapshot.debian.org; on the author's connection it delivered 36 kB/s
# and one package arrived corrupted (apt rejected it on its SHA-256), so the
# pinned-version approach replaced it. The versions are the ones that
# snapshot 20261123T000000Z resolved to.
FROM debian:trixie-slim@sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0

RUN apt-get -o Acquire::Retries=5 update \
 && DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::Retries=5 install -y --no-install-recommends \
      gcc=4:14.2.0-1 gcc-14=14.2.0-19 cpp-14=14.2.0-19 \
      libc6-dev=2.41-12+deb13u5 \
      clang=1:19.0-63 clang-19=1:19.1.7-3+b1 libllvm19=1:19.1.7-3+b1 \
      llvm=1:19.0-63 llvm-19=1:19.1.7-3+b1 \
      lld=1:19.0-63 lld-19=1:19.1.7-3+b1 \
      valgrind=1:3.24.0-3 make=4.4.1-2 \
 && rm -rf /var/lib/apt/lists/*

# Code runs as an unprivileged user with no home directory.
RUN useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin otb
USER 10001:10001
WORKDIR /work
