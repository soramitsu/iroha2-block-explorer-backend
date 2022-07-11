# API <!-- omit in toc -->

DTOs are described in [api.ts](./api.ts).

> **Warning**
>
> Most DTOs may contain `BigInt`s. If there are numbers greater than JavaScript native `number` can fit (`f64`), then native `JSON` decoder throws an error.
> 
> You should decode DTOs with a special JSON-decoder that works with `BigInt`s, e.g. [json-bigint](https://www.npmjs.com/package/json-bigint).

## Contents <!-- omit in toc -->

- [Endpoints](#endpoints)
  - [`GET` `/api/v1`](#get-apiv1)
  - [Blocks](#blocks)
    - [`GET` `/api/v1/blocks`](#get-apiv1blocks)
    - [`GET` `/api/v1/blocks/{height or hash}`](#get-apiv1blocksheight-or-hash)
  - [Transactions](#transactions)
    - [`GET` `/api/v1/transactions`](#get-apiv1transactions)
    - [`GET` `/api/v1/transactions/{hash}`](#get-apiv1transactionshash)
  - [Accounts](#accounts)
    - [`GET` `/api/v1/accounts`](#get-apiv1accounts)
    - [`GET` `/api/v1/accounts/{id}`](#get-apiv1accountsid)
  - [Assets](#assets)
    - [`GET` `/api/v1/assets`](#get-apiv1assets)
    - [`GET` `/api/v1/assets/{definition_id}/{account_id}`](#get-apiv1assetsdefinition_idaccount_id)
  - [Asset Definitions](#asset-definitions)
    - [`GET` `/api/v1/asset-definitions`](#get-apiv1asset-definitions)
    - [`GET` `/api/v1/asset-definitions/{id}`](#get-apiv1asset-definitionsid)
  - [Domains](#domains)
    - [`GET` `/api/v1/domains`](#get-apiv1domains)
    - [`GET` `/api/v1/domains/{id}`](#get-apiv1domainsid)
  - [Peers](#peers)
    - [`GET` `/api/v1/peer/peers`](#get-apiv1peerpeers)
    - [`GET` `/api/v1/peer/status`](#get-apiv1peerstatus)
  - [Roles](#roles)
    - [`GET` `/api/v1/roles`](#get-apiv1roles)
- [Id Transformation](#id-transformation)
- [Pagination Query Params](#pagination-query-params)

## Endpoints

### `GET` `/api/v1`

- **Description**: web server health check
- **Response**: 200 OK

### Blocks

- [`/blocks`](#get-apiv1blocks)
- [`/blocks/{height or hash}`](#get-apiv1blocksheight-or-hash)

#### `GET` `/api/v1/blocks`

- **Query**: [Pagination](#pagination-query-params)
- **Response**: `Paginated<BlockShallow>`

#### `GET` `/api/v1/blocks/{height or hash}`

- **Description**: get the block at the given height or hash
- **Params**: either `height` or `hash`

  |  Param   |   Type   |                    Description                     |
  | :------: | :------: | -------------------------------------------------- |
  | `height` |  `int`   | non-zero number indicating the height of the block |
  |  `hash`  | `string` | 32-byte hash hex of the block                      |

- **Response**: `Block` or `404`

### Transactions

- [`/transactions`](#get-apiv1transactions)
- [`/transactions/{hash}`](#get-apiv1transactionshash)

#### `GET` `/api/v1/transactions`

- **Query**: [Pagination](#pagination-query-params)
- **Response**: `Paginated<Transaction>`

#### `GET` `/api/v1/transactions/{hash}`

- **Params**:

  | Param  |   Type   |             Description             |
  | :----: | :------: | ----------------------------------- |
  | `hash` | `string` | 32-byte hash hex of the transaction |

- **Response**: `Transaction` or `404`

### Accounts

- [`/accounts`](#get-apiv1accounts)
- [`/accounts/{id}`](#get-apiv1accountsid)

#### `GET` `/api/v1/accounts`

- **Query**: [Pagination](#pagination-query-params)
- **Response**: `Paginated<Account>`

#### `GET` `/api/v1/accounts/{id}`

- **Params**:

  | Param |   Type   |      Description      |
  | :---: | :------: | --------------------- |
  | `id`  | `string` | The id of the account |

- **Response**: `Account` or `404`

See also: [Id Transformation](#id-transformation)

### Assets

- [`/assets`](#get-apiv1assets)
- [`/assets/{definition_id}/{account_id}`](#get-apiv1assetsdefinition_idaccount_id)

#### `GET` `/api/v1/assets`

- **Query**: [Pagination](#pagination-query-params)
- **Response**: `Paginated<Asset>`

#### `GET` `/api/v1/assets/{definition_id}/{account_id}`

- **Params**:

  |      Param      |   Type   |                Description                 |
  | :-------------: | :------: | ------------------------------------------ |
  | `definition_id` | `string` | The id of the asset definition             |
  |  `account_id`   | `string` | The id of the account the asset belongs to |

- **Response**: `Asset` or `404`

See also: [Id Transformation](#id-transformation)

### Asset Definitions

- [`/asset-definitions`](#get-apiv1asset-definitions)
- [`/asset-definitions/{id}`](#get-apiv1asset-definitionsid)

#### `GET` `/api/v1/asset-definitions`

- **Query**:[Pagination](#pagination-query-params)
- **Response**: `Paginated<AssetDefinition>`

#### `GET` `/api/v1/asset-definitions/{id}`

- **Params**:

  | Param |   Type   |          Description           |
  | :---: | :------: | ------------------------------ |
  | `id`  | `string` | The id of the asset definition |

- **Response**: `AssetDefinitionWithAccounts` or 404

See also: [Id Transformation](#id-transformation)

### Domains

- [`/domains`](#get-apiv1domains)
- [`/domains/{id}`](#get-apiv1domainsid)

#### `GET` `/api/v1/domains`

- **Query**: [Pagination](#pagination-query-params)
- **Response**: `Paginated<Domain>`

#### `GET` `/api/v1/domains/{id}`

- **Params**:

  | Param |   Type   |     Description      |
  | :---: | :------: | -------------------- |
  | `id`  | `string` | The id of the domain |

- **Response**: `Domain` or `404`

### Peer

- [`/peer/peers`](#get-apiv1peerpeers)
- [`/peer/status`](#get-apiv1peerstatus)

#### `GET` `/api/v1/peer/peers`

- **Response**: `Peer[]`

#### `GET` `/api/v1/peer/status`

- **Response**: `Status`

### Roles

#### `GET` `/api/v1/roles`

- **Response**: `Role[]`


## Id Transformation

IDs in path should be HTML-escaped:

|                     |       In DTO       |       In path       |
| ------------------- | ------------------ | ------------------- |
| Domain Id           | `wonderland`       | `wonderland`        |
| Account Id          | `alice@wonderland` | `alice@wonderland`  |
| Asset Definition Id | `rose#wonderland`  | `rose%23wonderland` |

## Pagination Query Params

| Param       | Type     | Default | Description     |
| ----------- | -------- | :-----: | --------------- |
| `page`      | `number` |    1    | Page number     |
| `page_size` | `number` |   15    | Page size limit |
