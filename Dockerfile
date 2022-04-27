FROM ekidd/rust-musl-builder:1.57.0 AS builder

COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY api.ts api.ts

RUN sudo chown -R rust:rust /home/rust

RUN cargo build --release

FROM alpine:3.14
RUN apk --no-cache add ca-certificates
COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/iroha2_explorer_web \
    /usr/local/bin/
CMD /usr/local/bin/iroha2_explorer_web