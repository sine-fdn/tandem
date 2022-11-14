# Tandem

<img src="https://avatars.githubusercontent.com/u/67928740?s=200&v=4" alt="SINE" height="150" align="right"/>

[Secure Multi-Party Computation (SMPC)](https://sine.foundation/library/002-smpc) is a cryptographic field with the goal of allowing two or more parties to cooperatively compute a result while keeping their inputs private.

Our aim at SINE is to make advanced technology, such as SMPC, available to as many companies as possible. We are especially keen to apply [SMPC for the exchange of sustainability data](https://sine.foundation/library/sine-is-partnering-with-wbcsd-to-decarbonise-the-economy).

Tandem, our SMPC engine, aims at fulfilling our vision by providing an easy to use framework to apply this technology wherever there is a need to share data privately, without a third-party trustee.

## Overview

This repository consists of four crates:

#### [`tandem`](tandem/)

This crate includes SINE's Secure Multi-Party Computation engine, Tandem. It is a Rust library, implementing [WRK17](https://acmccs.github.io/papers/p21-wangA.pdf) as a non-blocking [Finite State Machine](https://en.wikipedia.org/wiki/Finite-state_machine) (FSM). 

Please note that, for the time being, Tandem only supports __two-party__ computation. 

#### [`tandem_garble_interop`](tandem_garble_interop/)

The Tandem engine runs [Garbled Circuits](https://en.wikipedia.org/wiki/Garbled_circuit). As these are cumbersome to write, SINE provides a higher-level programming language: [Garble](https://github.com/sine-fdn/garble-lang). This crate provides helper functions for translating between the Tandem MPC engine circuit representation and the Garble language circuit representation and types.

#### [`tandem_http_client`](tandem_http_client/)

This crate provides an HTTP client to use the Tandem engine (against a running `tandem_http_server` server). This crate includes a CLI client, functions targetting WebAssembly and an [interactive notebook](https://mpc-notebook.fly.dev) to test Garble programs during development.

#### [`tandem_http_server`](tandem_http_server/)

This crate provides an HTTP server to use the Tandem engine (with some client). This crate can be used as either a library to construct a server with custom logic or as a binary to run a sample server based on [Rocket](https://rocket.rs/).

## Usage

### Basics

Every SMPC session run by the Tandem engine depends on the following data:

1. An MPC program, known to both client and server in plaintext;
2. The name of the function in the MPC program to be executed;
3. The _contributor_'s input, known only to the server (the first argument of the function);
4. The _evaluator_'s input, known only to the client (the second argument of the function).

The Tandem engine runs [Garbled Circuits](https://en.wikipedia.org/wiki/Garbled_circuit) and is agnostic as to what their source is. However, we recommend that all of the above be written in [Garble](https://github.com/sine-fdn/garble-lang), SINE's programming language, developed specifically for this end. To learn more about Garble and its syntax, take the [Language Tour](https://github.com/sine-fdn/garble/blob/main/language_tour.md).

In order for the Tandem engine to run, both a server and a client must already be set up and running. Please refer to the documentation on the [`tandem_http_server`](tandem_http_server/) and [`tandem_http_client`](tandem_http_client) crates to set them up according to your needs.

### Demo

The following is a demonstration of how to run Tandem with a simple function and inputs using the CLI provided by the `tandem_http_client` crate. Please follow these instructions in order.

#### 1. Starting the Server

Use the commands below to build, install and run the sample server provided by the `tandem_http_server` crate on `localhost:8000`:

```sh
cd tandem_http_server
cargo install --features="bin" --path .
tandem_http_server
```

This server acts as a simple 'echo server', which expects the contributor's input to be supplied by the client (as plaintext metadata). While not appropriate in any practical setting, this can be used to test different programs without having to re-deploy servers. This server will accept and execute all MPC programs sent by the client.

To know how to start a server with static configuration, refer to the [`tandem_http_server` documentation](tandem_http_server/README.md).

#### 2. Installing the Client's CLI

On a different terminal tab, use the commands below to build and install the CLI app provided by the `tandem_http_client` crate:

```sh
cd tandem_http_client
cargo install --features="bin" --path .
```

Once the CLI is installed, we can run Tandem using a command with the following structure:

```sh
tandem_http_client <PROGRAM> \
[OPTIONS] \
--function <FUNCTION> \
--input <INPUT> \
--metadata <METADATA>
```

The `[OPTIONS]` include the flag `--url`, allowing us to set the URL of the HTTP server to use. If none is provided, it defaults to the 'echo server' deployed by SINE: https://alpha.m1.sine.dev. (Run `tandem_http_client --help` for more information.)

#### 3. Running the Tandem Engine

We are now ready to gather the four pieces of data needed to run the Tandem engine. Below are the data to be used in this demonstration:

##### MPC Program

The following Garble program takes two unsigned `8 bit` numbers and outputs their sum:

```Rust
pub fn main(x: u8, y: u8) -> u8 {
    x + y
}
```

This program is stored in `tandem_http_client/tests/.add.garble.rs`.

##### Name of the Function

The function we want to call is `main`.

##### Contributor's Input

In this case, the contributor's input will be `110u8`.

##### Evaluator's Input

In this case, the evaluator's input will be `57u8`

##### Running Tandem

Use the following command to run the Tandem engine:

```sh
tandem_http_client tests/.add.garble.rs \
--function main \
--url http://localhost:8000/ \
--input 110u8 \
--metadata 57u8
```

Once Tandem has finished computing, the result will be printed in your terminal:

```sh
167u8
```

(If for some reason this is not what you see, please repeat the steps above and make sure nothing is missing. If that does not work, feel free to reach out.)

## Contributions

All contributions and suggestions are welcomed! Please open issues for that effect.
