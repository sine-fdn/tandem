# m1 http client

## Example Playground

The easiest way to quickly run Garble programs using an M1 server + client is by using the interactive playground provided by `index.html`:

1. (In the `m1_http_server` crate directory:) Start the M1 http server by running `cargo run --features="bin"`.
2. (In the `m1_http_client` crate directory:) Build the wasm blob and serve the local `index.html` file by running `./build.sh`.
3. Visit `http://localhost:9000/` and execute Garble functions (by default the m1 server will use the plaintext metadata as its input).

It might be necessary to clear the browser cache after rebuilding the wasm module.

<img width="1051" alt="Screenshot 2022-06-07 at 16 43 39" src="https://user-images.githubusercontent.com/358580/172409498-97117ebf-4700-4a0b-b52a-46f0b184c057.png">
