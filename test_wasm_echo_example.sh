#!/bin/bash

errorhandler () {
    kill $(jobs -p)
}
trap errorhandler ERR EXIT

cargo build --features "bin"
cargo run -p tandem_http_server --features "bin" &
sleep 2
if [[ "$OSTYPE" == "darwin"* ]]; then
    WASM_BINDGEN_TEST_TIMEOUT=300 wasm-pack test --release --headless \
    --safari tandem_http_client --test echo_server
else
    WASM_BINDGEN_TEST_TIMEOUT=300 wasm-pack test --release --headless \
    --chrome tandem_http_client --test echo_server
fi
