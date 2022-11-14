use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use tandem_http_client::{compute, MpcData, MpcProgram};

mod common;

fn mul_1(c: &mut Criterion) {
    let source = include_str!("./multiplications_setup/program.garble.rs");
    compute_circuit(source, 1, c);
}

fn mul_10(c: &mut Criterion) {
    let source = include_str!("./multiplications_setup/program.garble.rs");
    compute_circuit(source, 10, c);
}

fn multiplications_benchmark(c: &mut Criterion) {
    common::compile_server();

    mul_1(c);
    mul_10(c);
}

criterion_group!(benches, multiplications_benchmark);
criterion_main!(benches);

fn compute_circuit(source: &str, num_mul: u64, c: &mut Criterion) {
    let contrib_input = "_".to_string();

    let function = format!("mul_{num_mul}");

    let compilation_start = Instant::now();

    let program =
        MpcProgram::new(source.to_string(), function).expect("Could not parse source code");

    println!("Garble compilation took {:?}", compilation_start.elapsed());

    println!("Circuit has {}", program.report_gates());

    common::with_server("./benches/multiplications_setup", |connection_string| {
        let bench_id = format!("mul_{num_mul} tandem_http_client");

        c.bench_function(&bench_id, |b| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let eval_input = MpcData::from_string(&program, "42u64".to_string()).unwrap();

                    compute(
                        connection_string.clone(),
                        contrib_input.clone(),
                        program.clone(),
                        eval_input,
                    )
                    .await
                    .unwrap();
                })
        });
        Ok(())
    })
    .unwrap()
}
