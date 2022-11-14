use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use tandem_http_client::{compute, MpcData, MpcProgram};

mod common;

fn and_10(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "and", 10, c);
}

fn and_100(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "and", 100, c);
}

fn and_1000(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "and", 1000, c);
}

fn and_10000(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "and", 10000, c);
}

fn xor_10(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "xor", 10, c);
}

fn xor_100(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "xor", 100, c);
}

fn xor_1000(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "xor", 1000, c);
}

fn xor_10000(c: &mut Criterion) {
    let source = include_str!("./circuits_setup/program.garble.rs");
    compute_circuit(source, "xor", 10000, c);
}

fn circuits_benchmark(c: &mut Criterion) {
    common::compile_server();

    and_10(c);
    and_100(c);
    and_1000(c);
    and_10000(c);
    xor_10(c);
    xor_100(c);
    xor_1000(c);
    xor_10000(c);
}

criterion_group!(benches, circuits_benchmark);
criterion_main!(benches);

fn compute_circuit(source: &str, gates: &str, num_gates: u16, c: &mut Criterion) {
    let contrib_input = "_".to_string();

    let function = format!("{gates}_{num_gates}");

    let compilation_start = Instant::now();

    let program =
        MpcProgram::new(source.to_string(), function).expect("Could not parse source code");

    println!("Garble compilation took {:?}", compilation_start.elapsed());

    println!("Circuit has {}", program.report_gates());

    common::with_server("./benches/circuits_setup", |connection_string| {
        let bench_id = format!(
            "{} gates tandem_http_client {num_gates}",
            gates.to_uppercase()
        );
        c.bench_function(&bench_id, |b| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let eval_input =
                        MpcData::from_string(&program, format!("[true; {num_gates}]")).unwrap();

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
    .unwrap();
}
