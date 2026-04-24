FROM rust:1-bookworm AS builder
WORKDIR /homework
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev libsasl2-dev && rm -rf /var/lib/apt/lists/*
COPY services/aggregator services/aggregator
WORKDIR services/aggregator
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/services/aggregator/target/release/movie-aggregator /usr/local/bin/movie-aggregator
EXPOSE 8081
ENV AGGREGATOR_PORT=8081
ENTRYPOINT ["/usr/local/bin/movie-aggregator"]
