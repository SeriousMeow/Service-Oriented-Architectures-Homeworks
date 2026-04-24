FROM rust:1-bookworm AS builder
WORKDIR /homework
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY tests/integration tests/integration
WORKDIR /homework/tests/integration
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/tests/integration/target/release/movie-pipeline-integration /usr/local/bin/movie-pipeline-integration
ENTRYPOINT ["/usr/local/bin/movie-pipeline-integration"]
