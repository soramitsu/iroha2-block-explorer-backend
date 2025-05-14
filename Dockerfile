FROM rust:alpine3.21 AS builder

WORKDIR /app

RUN apk add musl-dev pkgconfig openssl-dev openssl-libs-static

# NOTE: this disregards `./rust-toolchain.toml`, but it's fine
COPY Cargo.lock Cargo.toml build.rs ./
COPY src src
RUN cargo fetch
RUN cargo build --release

FROM alpine:3.21
COPY --from=builder /app/target/release/iroha_explorer /usr/local/bin/
CMD iroha_explorer serve
