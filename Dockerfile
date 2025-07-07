FROM rust:alpine3.21 AS builder

WORKDIR /app

RUN apk add build-base pkgconfig openssl-dev openssl-libs-static git
COPY Cargo* rust-toolchain.toml build.rs ./
COPY src src
COPY .git .git
RUN cargo fetch --locked
RUN cargo build --release

FROM alpine:3.21

COPY --from=builder /app/target/release/iroha_explorer /usr/local/bin/

ARG UID=1001
ARG GID=1001

RUN addgroup -g $GID explorer \
 && adduser -D -H -u $UID -G explorer explorer

USER explorer

CMD iroha_explorer serve
