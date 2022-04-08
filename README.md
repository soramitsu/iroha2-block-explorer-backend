# iroha2-block-explorer-backend

```
$ cargo run -- -h
   Compiling iroha2_explorer_web v0.1.0 (/home/re/dev/iroha2-block-explorer-backend)
    Finished dev [unoptimized + debuginfo] target(s) in 6.46s
     Running `target/debug/iroha2_explorer_web -h`
iroha2_explorer_web 0.1.0
Iroha 2 Explorer Backend

USAGE:
    iroha2_explorer_web [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --client-config <client-config>    `iroha_client` JSON configuration path [default: client_config.json]
    -p, --port <port>                       [env: PORT=]  [default: 4000]
```

Check:

```
$ curl http://localhost:4000/status
Status { peers: 0, blocks: 1, txs: 1, uptime: Uptime(107.296s) }%
```

## API

DTOs are described at [api.ts](./api.ts).

- `GET /accounts`
  - Response: `Account[]`
- `GET /assets`
  - Response: `Asset[]`
- `GET /assets/definitions`
  - Response: `AssetDefinition[]`
- `GET /domains`
  - Response: `Domain[]`
- `GET /peer/peers`
  - Response: `Peer[]`
- _TODO `GET /peer/parameters`_
  - Reponse: ?
- `GET /peer/status`
  - Reponse: `Status`
- `GET /roles`
  - Response: `Role[]`
