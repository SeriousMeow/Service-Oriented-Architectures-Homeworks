FROM rust:1-bookworm AS builder
WORKDIR /homework
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev libsasl2-dev && rm -rf /var/lib/apt/lists/*
COPY schemas/avro /homework/schemas/avro
COPY services/common /homework/services/common
COPY services/producer /homework/services/producer
WORKDIR /homework/services/producer
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl libssl3 libsasl2-2 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/services/producer/target/release/warehouse-producer /usr/local/bin/warehouse-producer
EXPOSE 8080
ENV PRODUCER_PORT=8080
ENTRYPOINT ["/usr/local/bin/warehouse-producer"]
