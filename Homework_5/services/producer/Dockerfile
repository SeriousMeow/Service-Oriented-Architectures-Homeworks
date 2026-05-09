FROM rust:1-bookworm AS builder
WORKDIR /homework/services/producer
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev libsasl2-dev && rm -rf /var/lib/apt/lists/*
COPY schemas /homework/schemas
COPY services/producer /homework/services/producer
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl libssl3 libsasl2-2 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/services/producer/target/release/movie-producer /usr/local/bin/movie-producer
EXPOSE 8080
ENV PRODUCER_PORT=8080
ENTRYPOINT ["/usr/local/bin/movie-producer"]
