FROM rust:1.89-bookworm AS builder

WORKDIR /app
COPY proto ./proto
COPY services/flight ./services/flight

WORKDIR /app/services/flight
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/services/flight/target/release/flight-service /usr/local/bin/flight-service

EXPOSE 50051
CMD ["flight-service"]

