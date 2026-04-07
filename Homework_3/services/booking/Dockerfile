FROM rust:1.89-bookworm AS builder

WORKDIR /app
COPY proto ./proto
COPY openapi ./openapi
COPY services/booking ./services/booking

RUN cargo install oas3-gen --locked

WORKDIR /app/services/booking
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/services/booking/target/release/booking-service /usr/local/bin/booking-service

EXPOSE 8000
CMD ["booking-service"]

