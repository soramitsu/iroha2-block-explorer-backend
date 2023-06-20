# Iroha 2 Blockchain Explorer Backend

This readme provides an overview of the Iroha 2 blockchain explorer backend and instructions on how to install, run, and utilize the explorer's backend features.

## Installation

To set up the Iroha 2 blockchain explorer backend, follow these steps:

1. Install Rust.
2. For full functionality of the explorer backend, build [Iroha_RC.9](https://github.com/hyperledger/iroha/tree/ea45b5053018acd48340024800786ff5a3d0904d) and ensure it is running.

3. Build the explorer backend binary by running the following command:
```bash
cargo build --release 
```

4. Prepare Iroha Client config ([reference](https://github.com/hyperledger/iroha/blob/ea45b5053018acd48340024800786ff5a3d0904d/docs/source/references/config.md)). **Define target peer location here**.
Or copy the configuration file from  [explorer-deploy-dev-tool](https://github.com/0x009922/explorer-deploy-dev-tool) Config Files

## Run

To run the Iroha 2 blockchain explorer backend, execute the following command:

```bash
./target/release/iroha2_explorer_web \
  -c /path/to/client_config.json \
  -p 8080  # may be env PORT, default is 4000
```

or


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


## Check the API

Ensure that the explorer backend is functioning correctly by executing the following command:

```bash
> curl http://localhost:4000/api/v1
Welcome to Iroha 2 Block Explorer!
```

## API

Refer to [Block Explorer API](api.md).


## Tools

The following tools are available in conjunction with the Iroha 2 blockchain explorer:

- [genesis-gen](./tools/genesis-gen/README.md): Genesis generator (a tool to generate sample data).
- [explorer-deploy-dev-tool](https://github.com/0x009922/explorer-deploy-dev-tool): A tool for automating the deployment of Iroha and the explorer.


