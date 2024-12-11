#![cfg(target_arch = "wasm32")]

use tandem_http_client::{compute, MpcData, MpcProgram};
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test() {
    console_log::init().expect("Could not init console_log");
    let url = "http://127.0.0.1:8000";
    let source_code = "pub fn main(a: i32, b: u16) -> i32 { a + (b as i32) }".to_string();
    let function = "main".to_string();
    let program = MpcProgram::new(source_code, function).expect("Could not parse source code");
    let remote_input = "2i32";
    let my_input =
        MpcData::from_string(&program, "2u16".to_string()).expect("Could not parse input");
    let output = compute(url.to_string(), remote_input.to_string(), program, my_input).await;
    match output {
        Ok(output) => assert_eq!(output.to_literal_string(), "4"),
        Err(e) => panic!("{e:?}"),
    }
}
