FROM rust:1.93.1-alpine as builder

WORKDIR /build

RUN apk add --no-cache \
    pkgconfig \
    openssl-dev

RUN cargo install oas3-gen

COPY openapi ./openapi
COPY service ./service

RUN oas3-gen generate server-mod -i openapi/api.yaml -o service/src/api

RUN cd service && cargo build --release

FROM alpine:latest

RUN apk add --no-cache \
    ca-certificates \
    openssl

WORKDIR /app

COPY --from=builder /build/service/target/release/server /app/server

ENV PORT=8000

EXPOSE 8000

CMD ["./server"]

