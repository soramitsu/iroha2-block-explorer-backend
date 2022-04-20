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
> curl http://localhost:4000
Welcome to Iroha 2 Block Explorer!
```

## API

DTOs are described at [api.ts](./api.ts).

> **Warning**
>
> Some DTOs (most of them) may contain bigints. If there are numbers greater than JavaScript's native `number` can fit (`f64`), then native `JSON` decoder will throw an error. You should decode DTOs with some special JSON-decoder than is fine with bigints, e.g. https://www.npmjs.com/package/json-bigint

- `GET /` - web server health check. Returns 200 OK.

- **TODO** `GET /blocks`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: ?

- **TODO** `GET /blocks/{height}`

  - Params:

    - `height` - numeric

  - Response: ?

- **TODO** `GET /transactions`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: ?

- `GET /accounts`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Account>`

- `GET /accounts/{id}`

  - Params:

    - `id` - string. The id of the account.

  - Response: `Account` or 404

  - Also: [Id Transformation](#id-transformation)

- `GET /assets`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Asset>`

- `GET /assets/{definition_id}/{account_id}`

  - Params:

    - `definition_id` - string. The id of the asset definition.
    - `account_id` - string. The id of the account the asset belongs to.

  - Response: `Asset` or 404

  - Also: [Id Transformation](#id-transformation)

- `GET /asset-definitions`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<AssetDefinition>`

- **TODO** `GET /asset-definitions/{id}`

  - Params:

    - `id` - string. The id of the asset definition.

  - Response: `AssetDefinition` or 404

  - Also: [Id Transformation](#id-transformation)

- `GET /domains`

  - Query:

    - [Pagination](#pagination-query-params)

  - Response: `Paginated<Domain>`

- `GET /domains/{id}`

  - Params:

    - `id` - string. The id of the domain.

  - Response: `Domain` or 404

- `GET /peer/peers`

  - Response: `Peer[]`

- `GET /peer/status`

  - Reponse: `Status`

- `GET /roles`

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

Pagination is not yet implemented! For now it always returns the whole dataset. Query params will be ignored.
