# Iroha 2 Explorer Backend

This is the backend service for
the [Iroha 2 Block Explorer Web application](https://github.com/soramitsu/iroha2-block-explorer-web).
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

- `serve` to run the server
- `scan` to scan Iroha ones and save into a database (for troubleshooting)
- pass `--torii-urls`, `--account`, and `--account-private-key` options to configure connection to Iroha

For example:

```shell
# via ENVs; also could be passed as CLI args
export IROHA_EXPLORER_ACCOUNT=<account id>
export IROHA_EXPLORER_ACCOUNT_PRIVATE_KEY=<acount private key>

# At least one URL is required, the rest are for telemetry gathering
export IROHA_EXPLORER_TORII_URLS=http://localhost:8080,http://localhost:8081

iroha_explorer serve --port 4123
iroha_explorer scan ./scanned.sqlite
```

### OpenAPI Documentation

With the running server, open `/api/docs` path (e.g. `http://localhost:4123/api/docs`) for API documentation.

### Telemetry

Iroha Explorer supports gathering telemetry from multiple nodes. You have to provide API URLs (Torii URLs) of each peer for that:

```shell
iroha_explorer serve --torii-urls http://localhost:8080,http://localhost:8081,http://localhost:8082
```

The documentation of the telemetry endpoints is available in the OpenAPI Documentation.

### Logging

To configure logging, use `RUST_LOG` env var. For example:

```shell
RUST_LOG=iroha_explorer=debug,sqlx=debug
```

### Health check

```shell
test "$(curl -fsSL localhost:4000/api/health)" = "healthy" && echo OK || echo FAIL
```

### Serve with sample data

_Only available with `sample` feature enabled._

Serve sample data without connecting to Iroha:

```shell
cargo run --features sample -- serve-sample
```

> Note: peers telemetry data will be unavailable in this case.

## Compatibility and Versioning

Iroha Explorer aims to support only the latest version of Iroha. Currently it is `v2.0.0-rc.2.x`.

<!-- TODO: include a tip to run `iroha_explorer --version` to see the compatible Iroha version -->

For reference:

| Iroha               | Iroha Explorer Backend ([tags](https://github.com/soramitsu/iroha2-block-explorer-backend/tags)) | Iroha Explorer Web ([tags](https://github.com/soramitsu/iroha2-block-explorer-web/tags))                      |
| ------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `v2.0.0-rc.2.x`     | `v0.2.x`                                                                                         | _WiP, partial support: [`develop`](https://github.com/soramitsu/iroha2-block-explorer-web/tree/74c2e43cd6c8)_ |
| `v2.0.0-rc.1.x`[^1] | `v0.1.0`                                                                                         | `v0.1.0`                                                                                                      |

[^1]: Iroha versions `rc.1.3` and `rc.1.4` are not compatible with Explorer because of accidental breaking changes introduced in Iroha itself. Use version `rc.1.5`.
