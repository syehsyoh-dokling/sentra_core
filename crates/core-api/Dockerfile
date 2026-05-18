FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml ./Cargo.toml
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build -p core-api --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/core-api /usr/local/bin/core-api
COPY migrations ./migrations

EXPOSE 4000

CMD ["core-api"]

