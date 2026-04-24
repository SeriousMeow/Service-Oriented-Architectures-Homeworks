FROM rust:1-bookworm AS builder
WORKDIR /homework
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY services/exporter services/exporter
WORKDIR services/exporter
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/services/exporter/target/release/movie-exporter /usr/local/bin/movie-exporter
EXPOSE 8082
ENV EXPORTER_PORT=8082
ENTRYPOINT ["/usr/local/bin/movie-exporter"]
