FROM rust:1-bookworm AS builder
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev libsasl2-dev && rm -rf /var/lib/apt/lists/*
COPY services/common /homework/services/common
COPY services/consumer /homework/services/consumer
WORKDIR /homework/services/consumer
RUN /usr/local/cargo/bin/cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl libssl3 libsasl2-2 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /homework/services/consumer/target/release/warehouse-consumer /usr/local/bin/warehouse-consumer
EXPOSE 3000
ENV CONSUMER_PORT=3000
ENTRYPOINT ["/usr/local/bin/warehouse-consumer"]
