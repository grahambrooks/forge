FROM rust:1.78 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/fullstack-api /usr/local/bin/
EXPOSE 8080
CMD ["fullstack-api"]
