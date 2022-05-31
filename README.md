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

DTOs are described at [api.ts](./api.ts).

> **Warning**
>
> Some DTOs (most of them) may contain bigints. If there are numbers greater than JavaScript's native `number` can fit (`f64`), then native `JSON` decoder will throw an error. You should decode DTOs with some special JSON-decoder than is fine with bigints, e.g. https://www.npmjs.com/package/json-bigint

> **Important**
>
> All paths are prefixed with `/api/v1`

- **`GET`** `/`

  - Description: web server health check.

  - Response: 200 OK

- **`GET`** `/blocks`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<BlockShallow>`

- **`GET`** `/blocks/{height or hash}`

  - Params:

    - `height or hash` - non-zero number or 32-byte hash hex

  - Response: `Block` or 404

- **`GET`** `/transactions`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Transaction>`

- **`GET`** `/transactions/{hash}`

  - Query:

    - `hash` - 32-byte hash hex

  - Response: `Transaction` or 404

- **`GET`** `/accounts`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Account>`

- **`GET`** `/accounts/{id}`

  - Params:

    - `id` - string. The id of the account.

  - Response: `Account` or 404

  - Also: [Id Transformation](#id-transformation)

- **`GET`** `/assets`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Asset>`

- **`GET`** `/assets/{definition_id}/{account_id}`

  - Params:

    - `definition_id` - string. The id of the asset definition.
    - `account_id` - string. The id of the account the asset belongs to.

  - Response: `Asset` or 404

  - Also: [Id Transformation](#id-transformation)

- **`GET`** `/asset-definitions`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<AssetDefinition>`

- **`GET`** `/asset-definitions/{id}`

  - Params:

    - `id` - string. The id of the asset definition.

  - Response: `AssetDefinitionWithAccounts` or 404

  - Also: [Id Transformation](#id-transformation)

- **`GET`** `/domains`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Domain>`

- **`GET`** `/domains/{id}`

  - Params:

    - `id` - string. The id of the domain.

  - Response: `Domain` or 404

- **`GET`** `/peer/peers`

  - Response: `Peer[]`

- **`GET`** `/peer/status`

  - Reponse: `Status`

- **`GET`** `/roles`

  - Response: `Role[]`

### Id Transformation

They should be HTML-escaped.

|                     | In DTO             |       In path       |
| ------------------- | ------------------ | :-----------------: |
| Domain Id           | `wonderland`       |    `wonderland`     |
| Account Id          | `alice@wonderland` | `alice@wonderland`  |
| Asset Definition Id | `rose#wonderland`  | `rose%23wonderland` |

### Pagination Query Params

- `page=<number>` - page number. Default: 1.
- `page_size=<number>` - page size limit. Default: 15.


## Tools

- [genesis-gen](./tools/genesis-gen/README.md) - genesis generator.
