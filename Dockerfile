# FROM rust:1.60.0 as builder

# # Set environment variables
# ENV RUSTUP_HOME="/opt/rust"
# ENV CARGO_HOME="/opt/rust"
# ENV PATH="$PATH:$RUSTUP_HOME/bin"
# ENV CARGO_BUILD_DEP_INFO_BASEDIR="."


# # Build project
# RUN mkdir ${RUSTUP_HOME}
# WORKDIR ${RUSTUP_HOME}

# RUN adduser --disabled-password --gecos "" iroha && \
#    chown -R iroha ${RUSTUP_HOME}

# USER iroha


# COPY src src
# COPY Cargo.toml Cargo.toml
# COPY Cargo.lock Cargo.lock
# COPY api.ts api.ts
# #ENV RUSTC_WRAPPER=sccache
# #ENV CARGO_INCREMENTAL=0


# #RUN rustup default stable
# RUN rustup install stable
# RUN cargo build --release


# #CMD ["cargo", "run"]
# #CMD ["cargo", "build", "--release"]


# # RUN ./target/release/iroha2_explorer_web \
# #         -c /path/to/client_config.json \
# #         -p 8080
# # WORKDIR /





FROM rust:1.60.0
ENV RUSTUP_HOME="/opt/rust"

WORKDIR ${RUSTUP_HOME}

COPY . .
RUN rustup default stable

RUN cargo build --release

#RUN cargo install --path .

#CMD ["/usr/local/cargo/bin/myapp"]