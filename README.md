# Iroha 2 Explorer Backend

This is a backend service for the [web Explorer application](https://github.com/soramitsu/iroha2-block-explorer-web).
It is written in Rust and provides a classic HTTP-API way to observe data
in [Iroha 2](https://github.com/hyperledger/iroha).

Note that the current implementation is more of a draft and is not production ready. This implementation maintains an
in-memory normalised SQLite database that reflects the history of blocks (transactions, instructions) in Iroha and its
current world state (domains, accounts, assets). The database is re-created from scratch on each update in Iroha, which
is not suitable for a large scale. The rationale of using an SQL-based solution is that currently, Iroha Query API is
limited and cannot provide some features that are necessary for efficient Explorer implementation (e.g. selects and
joins). However, having an SQL-based querying allows creating a set of endpoints to serve the frontend needs
efficiently, allowing at least the frontend implementation _as desired_.

## Usage

Prerequisites: [Rust toolchain](https://rustup.rs/).

Install from this repo remotely:

```shell
cargo install --git https://github.com/soramitsu/iroha2-block-explorer-backend

iroha_explorer help
```

Or build and run locally from the repo with:

```shell
cargo run --release -- help
```

Use `help` for detailed usage documentation. In short:

- pass `--torii-url`, `--account`, and `--account-private-key` options to configure connection to Iroha
- use `serve` command to run the server, and `scan` command to scan Iroha

For example:

```shell
# via ENVs; also could be passed as CLI args
export ACCOUNT=<account id>
export ACCOUNT_PRIVATE_KEY=<acount private key>
export TORII_URL=http://localhost:8080

iroha_explorer serve --port 4123
iroha_explorer scan ./scanned.sqlite
```

With the running server, open `/scalar` path (e.g. `http://localhost:4123/scalar`) for API documentation.
