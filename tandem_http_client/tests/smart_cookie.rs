#![cfg(target_arch = "wasm32")]

use std::include_str;

use js_sys::Reflect;
use tandem_http_client::{compute, MpcData, MpcProgram};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_valid_signature() {
    console_log::init().expect("Could not init console_log");
    let url = "http://127.0.0.1:8000";

    let smart_cookie_prg = include_str!("smart_cookie_setup/program.garble.rs");

    let function = "init".to_string();
    let program = MpcProgram::new(smart_cookie_prg.to_string(), function)
        .expect("Could not parse source code");
    let metadata = "_";
    let my_input =
        MpcData::from_string(&program, "()".to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut user_state = compute(url.to_string(), metadata.to_string(), program, my_input)
        .await
        .unwrap_or_else(|e| panic!("{e}"));

    let function = "log_interest".to_string();
    let program = MpcProgram::new(smart_cookie_prg.to_string(), function)
        .expect("Could not parse source code");
    let metadata = "article4";
    let log_result = compute(
        url.to_string(),
        metadata.to_string(),
        program.clone(),
        user_state,
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"))
    .to_literal()
    .unwrap_or_else(|e| panic!("{e}"));
    let log_result = Reflect::get(&log_result, &"Enum".into()).unwrap_or_else(|e| panic!("{e:?}"));
    let log_result_enum_name = Reflect::get_u32(&log_result, 0).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(log_result_enum_name, JsValue::from("LogResult"));
    let log_result_enum_variant =
        Reflect::get_u32(&log_result, 1).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(log_result_enum_variant, JsValue::from("Ok"));
    let log_result = Reflect::get_u32(&log_result, 2).unwrap_or_else(|e| panic!("{e:?}"));
    let log_result = Reflect::get(&log_result, &"Tuple".into()).unwrap_or_else(|e| panic!("{e:?}"));
    let log_result = Reflect::get_u32(&log_result, 0).unwrap_or_else(|e| panic!("{e:?}"));
    user_state = MpcData::from_object(&program, log_result).unwrap_or_else(|e| panic!("{e}"));

    let function = "decide_ad".to_string();
    let program = MpcProgram::new(smart_cookie_prg.to_string(), function)
        .expect("Could not parse source code");
    let metadata = "_";
    let ad_decision = compute(url.to_string(), metadata.to_string(), program, user_state)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(ad_decision.to_literal_string().contains("Sports"));
}

#[wasm_bindgen_test]
async fn test_invalid_signature() {
    let url = "http://127.0.0.1:8000";

    let smart_cookie_prg = include_str!("smart_cookie_setup/program.garble.rs");

    let function = "init".to_string();
    let program = MpcProgram::new(smart_cookie_prg.to_string(), function)
        .expect("Could not parse source code");
    let metadata = "_";
    let my_input =
        MpcData::from_string(&program, "()".to_string()).unwrap_or_else(|e| panic!("{e}"));
    let user_state = compute(url.to_string(), metadata.to_string(), program, my_input)
        .await
        .unwrap_or_else(|e| panic!("{e}"));

    let function = "log_interest".to_string();
    let program = MpcProgram::new(smart_cookie_prg.to_string(), function)
        .expect("Could not parse source code");
    let metadata = "article5";
    let log_result = compute(
        url.to_string(),
        metadata.to_string(),
        program.clone(),
        user_state,
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"))
    .to_literal()
    .unwrap_or_else(|e| panic!("{e}"));
    let log_result = Reflect::get(&log_result, &"Enum".into()).unwrap_or_else(|e| panic!("{e:?}"));
    let log_result_enum_name = Reflect::get_u32(&log_result, 0).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(log_result_enum_name, JsValue::from("LogResult"));
    let log_result_enum_variant =
        Reflect::get_u32(&log_result, 1).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(log_result_enum_variant, JsValue::from("InvalidSignature"));
}
