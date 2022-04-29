FROM ekidd/rust-musl-builder:1.57.0 AS builder

COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY api.ts api.ts

RUN sudo chown -R rust:rust /home/rust && \
    cargo build --release

FROM alpine:3.14

ENV LOAD_DIR=/usr/local/bin/

RUN apk --no-cache add ca-certificates && \
    adduser --disabled-password --gecos "" iroha && \
    chown -R iroha ${LOAD_DIR}
    
USER iroha

COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/iroha2_explorer_web \
    ${LOAD_DIR}

CMD ${LOAD_DIR}iroha2_explorer_web