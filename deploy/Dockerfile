FROM rust:latest AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --bin pwright-server --bin pwright

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y curl dnsutils ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/pwright-server /usr/local/bin/
COPY --from=builder /build/target/release/pwright /usr/local/bin/

EXPOSE 50051
