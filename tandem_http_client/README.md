# Tandem HTTP Client

This crate provides an HTTP client acting as the evaluator and running the [Tandem engine](../README.md). It connects with an [HTTP server](../tandem_http_server/README.md) which is expected to act as the contributor in turn.

The evaluator always triggers the SMPC session, sending `metadata` alongside the evaluator's input to influence the server's choice of for the contributor's input.

## Overview

This crate includes
- [a CLI client](#cli-client);
- [functions targeting WebAssembly](#functions-targeting-webassembly);
- [an interactive playground](#playground) to test Garble programs during development.

## CLI Client

The CLI client provides a quick and easy way of running the Tandem engine. In order to use it, a Tandem HTTP server must already be running. If no server is specified, it will default to SINE's sample server at https://echo-server.sine.dev/.

Use the following commands to install the CLI client:

```sh
cd tandem_http_client
cargo install --features="bin" --path .
```

Once the CLI is installed, Tandem can be run using a command with the following structure:

```sh
tandem_http_client \
# Path to a Garble program file
<PROGRAM> \
# Options, including --url <URL> to set the URL of a Tandem http server.
[OPTIONS] \
# Name of the Garble function to be executed
--function <FUNCTION> \
# Garble input literal for this (local) party
--input <INPUT> \
# Metadata to send to the server (as plaintext) to influence the contributor's input
--metadata <METADATA>
```

These and further information can be found by running `tandem_http_client --help`.

Assuming that a Tandem HTTP server is listening on port `8000`, the following is an example usage of the CLI client:

```sh
tandem_http_client tests/.add.garble.rs \
--function main \
--url http://localhost:8000/ \
--input 110u8 \
--metadata 57u8
```

## Functions Targeting WebAssembly

This crate includes two functions targetting WebAssembly, allowing for an easy integration of the Tandem engine with JavaScript. For details on how the compilation from Rust to WebAssembly takes place see [WebAssembly's official doumentation](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm).

These functions are:

##### [`from_object`](./src/lib.rs)

Parses and type-checks a Garble literal in its JSON representation as Tandem data (`MpcData`).

##### [`to_literal`](./src/lib.rs)

Returns Tandem data (`MpcData`) as a Garble literal in its JSON representation.

## Playground

This crate provides also a simple web app to run and test Garble programs during development.

The playground is available [here](https://mpc-notebook.fly.dev/), running against a simple 'echo server'.

It can also be run locally, in which case the server can be based on configured handlers. Instructions on how to run the playground app locally can be found [below](#run-locally).

<p align="center">
  <img width="700" alt="playground sceenshot" src="https://user-images.githubusercontent.com/100690574/200596168-f36a44ca-e1c8-4ba4-a77f-f1ed2cfe01ec.png">
</p>

The playground has two main components which together provide the [data necessary to run the Tandem engine](../README.md#basics).

##### The Code Editor

<img width="300" alt="Code editor screenshot" align="right" src="https://user-images.githubusercontent.com/100690574/199686277-4390fdb3-0e47-48f9-943a-8c26f2a8d491.png">

Here you can write the Garble function you want to test. As an example, the code editor has a pre-written function that takes two `32 bit` signed numbers and returns their sum. To learn more about Garble and its syntax, take the [Language Tour](https://github.com/sine-fdn/garble-lang/blob/main/language_tour.md).

##### The Data Form

<img width="300" alt="Screenshot 2022-11-03 at 09 23 24" align="right" src="https://user-images.githubusercontent.com/100690574/199687300-418f7ed9-317e-48bd-a3a0-f29ae12634a4.png">

On the first field, write the __name of the function__ from the MPC program (written in the code editor) to execute. In the example provided, this should be `main`.

On the second field, write the __metadata__ that the client passes onto the server as plaintext to influence server's choice of contributor's input. If using a simple 'echo server', this metadata should be the contributor's input.

On the third field, write the __evaluator's input__, that is, the input coming from the client side and kept private. This should be a Garble literal. In the example provided, it should be a signed `32 bit` number, such as `-2i32`. __Please note__ that in Garble [the type suffix of a number must always be specified](https://github.com/sine-fdn/garble-lang/blob/main/language_tour.md#primitive-types).

Hitting `Compute` will start a Tandem session and return the computed value. In the example provided, it will be a signed `32 bit` number, such as `-1i32`. In case something goes wrong, an error message will be displayed in red to the right of the `Compute` button.

### Run Locally

Running the playground locally gives the user the possibility of choosing between a simple 'echo server' and a server based on configured handlers. Instructions on how to configure the latter can be found [here](../tandem_http_server/README.md#usage-as-binary:-static-configuration). Below are the instructions on how to run the playground locally.

#### 1. Starting the Server

Use the commands below to build and install the sample server provided by the `tandem_http_server`:

```sh
cd tandem_http_server
cargo install --features="bin" --path .
```

If you want to run a server based on handlers, make sure to include them in a `Tandem.toml` file. In the same directory, you must also include a `program.garble.rs` file with the Garble program to be run. Move into this directory before continuing.

Use the following command to run the sample server on `http://localhost:8000`:

```sh
tandem_http_server
```

#### 2. Serving the Local `index.html`

For ease of use, this crate includes a [`build.sh`](./build.sh) file comprising the commands necessary to build the wasm blob and serve the `index.html` file locally:

```sh
cd tandem_http_client
sh build.sh
```

The playground client will be served on `http://localhost:9000`.

It might be necessary to clear the browser cache after rebuilding the wasm module.

#### Using the Playground with a Server Based on Handlers

When using a server based on handlers, make sure to pass as metadata, not the contributor's input, but the `key` associated to that input as specified in the the `Tandem.toml` file (e.g., `_`).

Also, make sure that the Garble program in `program.garble.rs` is exactly the same as that in the code editor. __Please note__ that whitespaces are enough for these to be considered different.
