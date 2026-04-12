FROM rust:1.88-bookworm AS builder

# musl-tools provides `musl-gcc` needed by the C build scripts in the
# tree-sitter grammar crates that symgraph pulls in. Without it the
# musl cross-compile fails with "tool not found: <triple>-gcc".
RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY forge/ ./forge/

# Build glibc-linked binary (Debian, Ubuntu, Fedora)
RUN cd forge && cargo build --release

# Build musl-linked static binary (Alpine)
# Detect architecture for correct musl target. We point `cc-rs` at
# `musl-gcc` for both triples so the tree-sitter grammar build scripts
# don't look for `<triple>-gcc` binaries that Debian doesn't ship.
RUN ARCH="$(dpkg --print-architecture)" && \
    case "$ARCH" in \
      amd64)  TARGET="x86_64-unknown-linux-musl" ;; \
      arm64)  TARGET="aarch64-unknown-linux-musl" ;; \
      *)      echo "Unsupported arch: $ARCH" && exit 1 ;; \
    esac && \
    rustup target add "$TARGET" && \
    cd forge && \
    CC_x86_64_unknown_linux_musl=musl-gcc \
    CC_aarch64_unknown_linux_musl=musl-gcc \
    cargo build --release --target "$TARGET" && \
    cp "target/$TARGET/release/forge" target/release/forge-musl

FROM scratch
COPY --from=builder /build/forge/target/release/forge /forge-gnu
COPY --from=builder /build/forge/target/release/forge-musl /forge-musl
