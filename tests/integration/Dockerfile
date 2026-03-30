FROM rust:latest

RUN apt-get update && apt-get install -y protobuf-compiler curl dnsutils && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build integration tests
RUN cargo test -p pwright-integration-tests --no-run 2>&1 | tail -5
