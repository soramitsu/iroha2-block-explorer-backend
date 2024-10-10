# Iroha 2 Blockchain Explorer Backend

This readme provides an overview of the Iroha 2 blockchain explorer backend and instructions on how to install, run, and utilize the explorer's backend features.

## Demo

Check out the Hyperledger Iroha 2 Blockchain Explorer Demo video [here](https://www.youtube.com/watch?v=aze3jwW6d-U).

## Installation

To set up the Iroha 2 blockchain explorer backend, follow these steps:

1. Install Rust `1.70.0`
2. For full functionality of the explorer backend, build [Iroha `v2.0.0-pre-rc.9`](https://github.com/hyperledger/iroha/tree/ea45b5053018acd48340024800786ff5a3d0904d) and ensure it is running. You can find the build instructions for Iroha [here](https://hyperledger.github.io/iroha-2-docs/guide/build.html).

3. Build the explorer backend binary:

```bash
cargo build --release
```

4. To prepare the Iroha client configuration, you have two options:

   **Option 1:** Define the client configuration manually.

   To configure Iroha client, refer to [Configuration Reference](https://github.com/hyperledger/iroha/blob/ea45b5053018acd48340024800786ff5a3d0904d/docs/source/references/config.md) or [Iroha documentation](https://hyperledger.github.io/iroha-2-docs/guide/configure/client-configuration.html).

   **Option 2:** Copy the configuration file
   from [explorer-deploy-dev-tool](https://github.com/0x009922/explorer-deploy-dev-tool).

## Running the Backend

To run the Iroha 2 blockchain explorer backend, you have two options:

**Option 1:**
Execute the following command in your terminal:

```bash
./target/release/iroha2_explorer_web \
  -c /path/to/client_config.json \
  -p 8080  # may be env PORT, default is 4000
```

**Option 2:**
Alternatively, you can use the `cargo run` command with additional options.
This command will display the help information, including the available options and their descriptions.

```bash
cargo run -- -h
```

Here is a breakdown of the options you can use:

- `-c, --client-config <CLIENT_CONFIG>`: Specifies the path to the `iroha_client` JSON configuration file. The default path is set to `client_config.json` if not provided explicitly.

- `-h, --help`: Prints the help information, which provides an overview of the available options.

- `-p, --port <PORT>`: Allows you to specify the port number on which the server will listen. You can set the port by providing the value after the flag, for example, `-p 8080`. If you don't provide this flag, the default 4000 port will be used. Additionally, you can set the `PORT` environment variable to specify the port.

- `-V, --version`: Prints the version information of the Iroha 2 Explorer Backend.

Feel free to adjust the command and options according to your specific setup and requirements.

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

| Tool Name                                                                        | Description                                                                                                                                                                                                                                                                      |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [genesis-gen](./tools/genesis-gen/README.md)                                     | Genesis generator (a tool to generate sample data).                                                                                                                                                                                                                              |
| [explorer-deploy-dev-tool](https://github.com/0x009922/explorer-deploy-dev-tool) | A tool for automating the deployment of Iroha and the explorer. ( This tool is provided as a reference for setting up the project locally, but please note that it may be out of date and its functionality might not align with the current version of Iroha and the Explorer) |
