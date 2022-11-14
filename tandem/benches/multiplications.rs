use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};

use tandem_garble_interop::{check_program, compile_program, serialize_input, Role};

fn multiplications_benchmark(c: &mut Criterion) {
    mul(1, MUL_1, c);
    mul(10, MUL_10, c)
}

criterion_group!(benches, multiplications_benchmark);
criterion_main!(benches);

fn mul(num_mul: u64, garble_prg: &str, c: &mut Criterion) {
    let typed_prg = check_program(garble_prg).unwrap();

    let compilation_start = Instant::now();

    let function = format!("mul_{num_mul}");

    let circuit = compile_program(&typed_prg, &function).unwrap();

    let input_a = serialize_input(Role::Contributor, &typed_prg, &circuit.fn_def, "42u64").unwrap();
    let input_b = serialize_input(Role::Evaluator, &typed_prg, &circuit.fn_def, "42u64").unwrap();

    println!("Garble compilation took {:?}", compilation_start.elapsed());

    println!("Circuit has {}", circuit.info_about_gates);

    let bench_id = format!("mul_{num_mul} tandem");

    c.bench_function(&bench_id, |b| {
        b.iter(|| tandem::simulate(&circuit.gates, &input_a, &input_b).unwrap())
    });
}

const MUL_1: &str = "
pub fn mul_1(a: u64, b: u64) -> u64 {
  a * b
}
";

const MUL_10: &str = "
pub fn mul_10(a: u64, b: u64) -> u64 {
  a * b * b * b * b * b * b * b * b * b * b
}
";
