FROM ekidd/rust-musl-builder AS builder

COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY api.ts api.ts

RUN sudo chown -R rust:rust /home/rust

# Build our application.
RUN cargo build --release

# Now, we need to build our _real_ Docker container, copying in `iroha2_explorer_web`.
FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/iroha2_explorer_web \
    /usr/local/bin/
CMD /usr/local/bin/iroha2_explorer_web