use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use tandem_http_client::{compute, MpcData, MpcProgram};

mod common;

fn credit_scoring_benchmark(c: &mut Criterion) {
    common::compile_server();

    let credit_scorer_input = "scoring_algorithm1".to_string();

    let credit_scoring_prg =
        include_str!("../../tandem_http_client/tests/credit_scoring_setup/program.garble.rs")
            .to_string();

    let function = "compute_score".to_string();

    let compilation_start = Instant::now();

    let program =
        MpcProgram::new(credit_scoring_prg, function).expect("Could not parse source code");

    println!("Garble compilation took {:?}", compilation_start.elapsed());

    println!("Circuit has {}", program.report_gates());

    common::with_server("./tests/credit_scoring_setup", |connection_string| {
        c.bench_function("credit scoring tandem_http_client", |b| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let user_input = MpcData::from_string(&program, USER.to_string()).unwrap();

                    compute(
                        connection_string.to_string().clone(),
                        credit_scorer_input.clone(),
                        program.clone(),
                        user_input,
                    )
                    .await
                    .unwrap();
                })
        });
        Ok(())
    })
    .unwrap()
}

criterion_group!(benches, credit_scoring_benchmark);
criterion_main!(benches);

const USER: &str = "User {
    age: 37u8,
    income: 5500u32,
    account_balance: 25000i64,
    current_loans: 60000u64,
    credit_card_limit: 1000u32,
    ever_bankrupt: false,
    loan_payment_failures: 0u8,
    credit_payment_failures: 2u8,
    surety_income: 5000u32,
}";
