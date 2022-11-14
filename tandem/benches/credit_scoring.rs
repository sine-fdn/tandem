use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};

use tandem_garble_interop::{check_program, compile_program, serialize_input, Role};

fn credit_scoring_benchmark(c: &mut Criterion) {
    let credit_scoring = include_str!("../tests/credit_scoring_setup/credit_scoring.garble.rs");

    let typed_prg = check_program(credit_scoring).unwrap();

    let compilation_start = Instant::now();

    let circuit = compile_program(&typed_prg, "compute_score").unwrap();

    println!("Garble compilation took {:?}", compilation_start.elapsed());
    println!("Circuit has {}", circuit.info_about_gates);

    let credit_scorer_input = serialize_input(
        Role::Contributor,
        &typed_prg,
        &circuit.fn_def,
        SCORING_ALGORITHM,
    )
    .unwrap();

    let user_input = serialize_input(Role::Evaluator, &typed_prg, &circuit.fn_def, USER).unwrap();

    c.bench_function("credit scoring tandem", |b| {
        b.iter(|| {
            tandem::simulate(&circuit.gates, &credit_scorer_input, &user_input).unwrap();
        })
    });
}

criterion_group!(benches, credit_scoring_benchmark);
criterion_main!(benches);

const SCORING_ALGORITHM: &str = "
ScoringAlgorithm {
    age_score: [
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 18i64,
            },
            Points { inc: 0i32 },
        ),
        MatchClause::Range(
            Range {
                min: 18i64,
                max: 35i64,
            },
            Points { inc: 50i32 },
        ),
        MatchClause::Range(
            Range {
                min: 35i64,
                max: 65i64,
            },
            Points { inc: 100i32 },
        ),
        MatchClause::Range(
            Range {
                min: 65i64,
                max: 120i64,
            },
            Points { inc: 50i32 },
        ),
    ],
    income_score: [
        MatchClause::Range(
            Range {
                min: 2000i64,
                max: 5000i64,
            },
            Points { inc: 50i32 },
        ),
        MatchClause::Range(
            Range {
                min: 5000i64,
                max: 10000i64,
            },
            Points { inc: 100i32 },
        ),
        MatchClause::Range(
            Range {
                min: 10000i64,
                max: 999999999i64,
            },
            Points { inc: 200i32 },
        ),
        MatchClause::None,
    ],
    account_balance_score: [
        MatchClause::Range(
            Range {
                min: -999999999i64,
                max: 0i64,
            },
            Points { inc: -100i32 },
        ),
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 5000i64,
            },
            Points { inc: 0i32 },
        ),
        MatchClause::Range(
            Range {
                min: 5000i64,
                max: 10000i64,
            },
            Points { inc: 50i32 },
        ),
        MatchClause::Range(
            Range {
                min: 10000i64,
                max: 999999999i64,
            },
            Points { inc: 200i32 },
        ),
    ],
    current_loans_score: [
        MatchClause::Range(
            Range {
                min: 500000i64,
                max: 100000000i64,
            },
            Points { inc: 0i32 },
        ),
        MatchClause::Range(
            Range {
                min: 100000i64,
                max: 500000i64,
            },
            Points { inc: 150i32 },
        ),
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 100000i64,
            },
            Points { inc: 300i32 },
        ),
        MatchClause::None,
    ],
    credit_card_score: [
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 10000i64,
            },
            Points { inc: 100i32 },
        ),
        MatchClause::None,
        MatchClause::None,
        MatchClause::None,
    ],
    bankruptcy_score: [
        MatchClause::Bool(true, Points { inc: -100i32 }),
        MatchClause::Bool(false, Points { inc: 50i32 }),
    ],
    loan_payment_history_score: [
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 3i64,
            },
            Points { inc: 0i32 },
        ),
        MatchClause::Range(
            Range {
                min: 3i64,
                max: 6i64,
            },
            Points { inc: -100i32 },
        ),
        MatchClause::None,
        MatchClause::None,
    ],
    credit_payment_history_score: [
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 1i64,
            },
            Points { inc: 0i32 },
        ),
        MatchClause::Range(
            Range {
                min: 1i64,
                max: 3i64,
            },
            Points { inc: -100i32 },
        ),
        MatchClause::Range(
            Range {
                min: 3i64,
                max: 6i64,
            },
            Points { inc: -200i32 },
        ),
        MatchClause::None,
    ],
    surety_income_score: [
        MatchClause::Range(
            Range {
                min: 0i64,
                max: 1000i64,
            },
            Points { inc: -50i32 },
        ),
        MatchClause::Range(
            Range {
                min: 1000i64,
                max: 5000i64,
            },
            Points { inc: 0i32 },
        ),
        MatchClause::Range(
            Range {
                min: 5000i64,
                max: 10000i64,
            },
            Points { inc: 100i32 },
        ),
        MatchClause::None,
    ],
    score_limits: ScoreLimits {
        min: 0i32,
        max: 1000i32,
    },
}";

const USER: &str = "
User {
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
