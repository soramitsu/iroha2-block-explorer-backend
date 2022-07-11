# iroha2-block-explorer-backend

```
$ cargo run -- -h
iroha2_explorer_web 0.1.0
Iroha 2 Explorer Backend

USAGE:
    iroha2_explorer_web [OPTIONS]

OPTIONS:
    -c, --client-config <CLIENT_CONFIG>
            `iroha_client` JSON configuration path [default: client_config.json]

    -h, --help
            Print help information

    -p, --port <PORT>
            [env: PORT=] [default: 4000]

    -V, --version
            Print version information
```

Check:

```bash
> curl http://localhost:4000
Welcome to Iroha 2 Block Explorer!
```

## Deploy

- [Install Rust](https://www.rust-lang.org/tools/install)
- Build binary:

  ```bash
  cargo build --release
  ```

- Prepare Iroha Client config ([reference](https://github.com/hyperledger/iroha/blob/ea45b5053018acd48340024800786ff5a3d0904d/docs/source/references/config.md)). **Define target peer location here**.

- Run web server:

  ```bash
  ./target/release/iroha2_explorer_web \
      -c /path/to/client_config.json \
      -p 8080 # may be env PORT, default is 4000
  ```

## API

Refer to [Block Explorer API](api.md).

## Tools

- [genesis-gen](./tools/genesis-gen/README.md) - genesis generator