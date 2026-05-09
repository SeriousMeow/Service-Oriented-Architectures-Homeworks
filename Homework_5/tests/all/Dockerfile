FROM rust:1-bookworm AS builder
WORKDIR /homework
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY tests/all tests/all
WORKDIR /homework/tests/all
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/tests/all/target/release/movie-pipeline-all-test /usr/local/bin/movie-pipeline-all-test
ENTRYPOINT ["/usr/local/bin/movie-pipeline-all-test"]
