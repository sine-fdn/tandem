# Tandem HTTP Server

This crate provides an HTTP server acting as the contributor and running the [Tandem engine](../README.md). A connecting HTTP client is expected to act as the evaluator.

To learn how to use the provided server, refer to the [Usage](#usage) section below.

The server participates in the Tandem protocol execution and provides the contributor's input, configured through the plaintext metadata supplied by the client.

## Protocol description

The HTTP server effectively implements the following protocol (internally called "`dialog`") [h/t @spacejam]:

```Rust
server::dialog(
    engine_id: String,
    last_durably_received_offset: Option<u32>,
    messages: MessageLog,
) -> Result<(MessageLog, Option<u32>), Error>
```

Meaning: a HTTP client communicates with an engine which is identified by an `engine_id`. Upon each interaction with the engine, the optional message offset `last_durably_received_offset` commits to previously received messages.
Commiting to a message means: the calling client has successfully received and processed all messages _up to including_ the given offset.
The server is then expected to no longer return messages with a message id lower or equal to `last_durably_received_offset`.
Secondly, the server accepts a vector of new messages which it will process subsequently.

The result of the HTTP call is a tuple containing as first element
- a vector of messsages to be processed by the *calling* party,
- plus an optional message offset commitment. The semantics of the latter is the same as for `last_durably_received_offset` but for messages received from the calling client

## Description of the endpoints

| Endpoint | Semantics |
|----------|-------------------------------------------------------------------------|
| `POST /` | Receives a JSON struct of type `NewSession` and returns the `engine_id` |
| `POST /<engine_id>?[last_durably_received_offset=<offset>]` | Implementation of the `dialog` protocol as explained above |

## Usage

This crate can be used as either a __library__ or a __binary__. As a library, it provides a [`build`](src/lib.rs) function, which can be used to construct a server with
custom logic for choosing its input. As a binary, it provides a sample server based on [Rocket](https://rocket.rs).

To use this crate as a binary, it must be compiled with the `bin` feature. Use the following command for that effect:
```sh
cd tandem_http_server
cargo build --features="bin"
```

### Usage as Binary: Static Configuration

The server binary supports two modes of execution:

If the server is started __without any configuration__, it acts as a simple 'echo server' and expects the contributor's input to be supplied by the client (as plaintext metadata). This can be used to test different programs without re-deploying servers.

Alternatively, a __static configuration__ can be provided during server startup, through a `Tandem.json` or `Tandem.toml` file. This file describes which MPC function and which contributor input to use, based on the plaintext metadata supplied by the client. This file must be stored in the directory from which the server is started. The directory must also contain a file named `program.garble.rs` with the MPC program to run on the Tandem engine.

##### Example `Tandem.toml`

Consider the following Garble program:

```Rust
pub fn mul_1(a: u64, b: u64) -> u64 {
    a * b
}

pub fn mul_10(a: u64, b: u64) -> u64 {
    a * b * b * b * b * b * b * b * b * b * b
}
```

This program is stored in a [`program.garble.rs`](../tandem_http_client/benches/multiplications_setup/program.garble.rs) file. In the same directory, we have a [`Tandem.toml`](../tandem_http_client/benches/multiplications_setup/Tandem.toml) file with the following content:

```toml
[handlers.mul_1]
_ = "42u64"

[handlers.mul_10]
_ = "42u64"
```

The `[section name]` consists always of `handlers.` followed by the name of the function to which the the contributor input refers.

In each section, the `key` is the plaintext metadata that the client will supply to influence the server's choice of the contributor's input; the value is a string with the Garble literal that will used as the contributor's input.

Since some TOML parsers have trouble with empty strings as keys, our convention is to use `_` as plaintext metadata if the choice of the server's input is entirely left up to the server.

In this example, the server will always take `42u64` as the contributor's input. If we wanted to give further possibilities to the server, we could do it by adding `key = value` pairs to the relevant section, like so:

```toml
[handlers.mul_1]
_ = "42u64"
contrib2 = "100u64"
contrib3 = "200u64"
```

With this `Tandem.toml` file, the client would be able to chose between `_`, `contrib2` and `contrib3` when running Tandem with function `mul_1`. Note that the contributor's input would still remain hidden from the client, who only has knows the key associated with it.

For more realistic and complex examples of how such `Tandem.toml` files might be built and used, please refer to the [smart cookies](../tandem_http_client/tests/smart_cookie_setup/) and [credit scoring](../tandem_http_client/tests/credit_scoring_setup/) examples.

### Usage as Binary: Rocket Configuration

As the server is based on the [Rocket](https://rocket.rs) framework, it is possible to configure it according to the official [Rocket documentation](https://rocket.rs/v0.5-rc/guide/configuration/#configuration).

To have the server listen at port `8080`, for instance, one could, add a `Rocket.toml` file to the directory from which the server will be started with the following content:

```toml
[global]
port = 8080
```

Alternatively, one could also pass it as an environment variable when starting the server:

```sh
ROCKET_PORT=8080 tandem_http_server
```

This crate includes the possibility of configuring CORS via Rocket configuration. This too can be done with a `Rocket.toml` file or with an environment variable:

```toml
[global]
origins = "[\"example.com\", \"another_example.com\"]"
```

```sh
ROCKET_ORIGINS="example.com","another_example.com"
```

Local origins (`http://localhost` and `http://127.0.0.1`) are allowed by default. If no origins are specified, the CORS configuration defaults to "*".
