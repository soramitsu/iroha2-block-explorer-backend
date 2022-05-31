FROM  nwtgck/rust-musl-builder:1.60.0 AS builder

COPY  src src
COPY  tools tools
COPY  Cargo.toml Cargo.toml
COPY  Cargo.lock Cargo.lock
COPY  api.ts api.ts

RUN   cargo build --release

FROM  alpine:3.16

ENV   LOAD_DIR=/usr/local/bin/

RUN   apk --no-cache add ca-certificates && \
      adduser --disabled-password --gecos "" iroha
    
COPY  --from=builder \
      /home/rust/src/target/x86_64-unknown-linux-musl/release/iroha2_explorer_web \
      ${LOAD_DIR}

CMD   ${LOAD_DIR}iroha2_explorer_web

USER  iroha