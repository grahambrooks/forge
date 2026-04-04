FROM rust:1.86-bookworm AS builder
WORKDIR /build
COPY forge/ ./forge/
RUN cd forge && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends git ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/forge/target/release/forge /usr/local/bin/forge
COPY tests/integration/fixtures /fixtures
COPY tests/integration/run-tests.sh /run-tests.sh
RUN chmod +x /run-tests.sh

# Initialize a git repo in full-stack fixture so the git scanner has something to work with
RUN cd /fixtures/full-stack && \
    git init && \
    git config user.email "test@test.com" && \
    git config user.name "Test" && \
    git add -A && \
    git commit -m "init" && \
    git checkout -b feature/payments

CMD ["/run-tests.sh"]
