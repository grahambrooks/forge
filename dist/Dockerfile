FROM rust:1.86-bookworm AS builder
WORKDIR /build
COPY forge/ ./forge/

# Build glibc-linked binary (Debian, Ubuntu, Fedora)
RUN cd forge && cargo build --release

# Build musl-linked static binary (Alpine)
# Detect architecture for correct musl target
RUN ARCH="$(dpkg --print-architecture)" && \
    case "$ARCH" in \
      amd64)  TARGET="x86_64-unknown-linux-musl" ;; \
      arm64)  TARGET="aarch64-unknown-linux-musl" ;; \
      *)      echo "Unsupported arch: $ARCH" && exit 1 ;; \
    esac && \
    rustup target add "$TARGET" && \
    cd forge && cargo build --release --target "$TARGET" && \
    cp "target/$TARGET/release/forge" target/release/forge-musl

FROM scratch
COPY --from=builder /build/forge/target/release/forge /forge-gnu
COPY --from=builder /build/forge/target/release/forge-musl /forge-musl
