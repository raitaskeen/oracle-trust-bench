# Image in which all C code (references, shims, seeded faults and
# candidates) is compiled and run. See ADR-003 (sandbox) and ADR-005
# (constant-time method).
#
# Reproducibility:
# - the base image is pinned by digest (Debian 13.8, trixie-slim);
# - packages come from one dated snapshot of the Debian archive, so a
#   rebuild installs the same package versions;
# - the built image is itself referenced by its ID in run.json.
FROM debian:trixie-slim@sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0

ARG SNAPSHOT=20261123T000000Z

# Package lists are signed; the archive keyring ships in the base image.
# Check-Valid-Until is off because snapshot Release files have expired dates.
RUN printf '%s\n' \
      "Types: deb" \
      "URIs: http://snapshot.debian.org/archive/debian/${SNAPSHOT}" \
      "Suites: trixie trixie-updates" \
      "Components: main" \
      "Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg" \
      "" \
      "Types: deb" \
      "URIs: http://snapshot.debian.org/archive/debian-security/${SNAPSHOT}" \
      "Suites: trixie-security" \
      "Components: main" \
      "Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg" \
      > /etc/apt/sources.list.d/debian.sources \
 && apt-get -o Acquire::Check-Valid-Until=false update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      gcc libc6-dev clang llvm lld valgrind make \
 && rm -rf /var/lib/apt/lists/*

# Code runs as an unprivileged user with no home directory.
RUN useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin otb
USER 10001:10001
WORKDIR /work
