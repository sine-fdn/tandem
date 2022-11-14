#![cfg(target_arch = "wasm32")]

use tandem_http_client::{compute, MpcData, MpcProgram};
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test() {
    console_log::init().expect("Could not init console_log");
    let url = "http://127.0.0.1:8000".to_string();

    let metadata = "scoring_algorithm1".to_string();

    let credit_scoring_prg = include_str!("credit_scoring_setup/program.garble.rs").to_string();
    let function = "compute_score".to_string();
    let program =
        MpcProgram::new(credit_scoring_prg, function).expect("Could not parse source code");

    let my_input =
        MpcData::from_string(&program, "User {age: 37u8, income: 5500u32, account_balance: 25000i64, current_loans: 60000u64, credit_card_limit: 1000u32, ever_bankrupt: false, loan_payment_failures: 0u8, credit_payment_failures: 2u8, surety_income: 5000u32}".to_string()).unwrap_or_else(|e| panic!("{e}"));

    let score = compute(url, metadata, program, my_input)
        .await
        .unwrap_or_else(|e| panic!("{e}"));

    assert_eq!(score.to_literal_string(), "Score::Good(85u8)")
}
