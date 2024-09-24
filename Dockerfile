FROM  nwtgck/rust-musl-builder:1.81.0 AS builder

COPY  src src
COPY  Cargo.toml Cargo.toml
COPY  Cargo.lock Cargo.lock

RUN   cargo build --release

FROM  alpine:3.16

ENV   LOAD_DIR=/usr/local/bin/

RUN   apk --no-cache add ca-certificates && \
      adduser --disabled-password --gecos "" iroha
    
COPY  --from=builder \
      /home/rust/src/target/x86_64-unknown-linux-musl/release/iroha_explorer \
      ${LOAD_DIR}

CMD   ${LOAD_DIR}iroha_explorer

USER  iroha