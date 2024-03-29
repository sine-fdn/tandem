#!/bin/bash

errorhandler () {
    kill $(jobs -p)
    rm ./Tandem.toml
    rm ./Rocket.toml
    rm -r ./garble_programs
}
trap errorhandler ERR EXIT

cargo build --features "bin"
cargo run -p tandem_http_server --features "bin" &
cp -r ./tandem_http_client/tests/smart_cookie_setup/* ./
sleep 10
if [[ "$OSTYPE" == "darwin"* ]]; then
    WASM_BINDGEN_TEST_TIMEOUT=300 wasm-pack test --release --headless \
    --firefox tandem_http_client --test smart_cookie
else
    WASM_BINDGEN_TEST_TIMEOUT=300 wasm-pack test --release --headless \
    --chrome tandem_http_client --test smart_cookie
fi
