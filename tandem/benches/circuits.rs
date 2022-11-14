use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tandem::{Circuit, Gate};

fn and(iterations: u32) -> Result<(), tandem::Error> {
    let mut gates = vec![Gate::InContrib];
    let output_gates = vec![iterations * 2];
    for i in 0..iterations {
        gates.append(&mut vec![Gate::InEval, Gate::And(i * 2, i * 2 + 1)]);
    }

    let program = Circuit::new(gates, output_gates);

    let input_a = vec![true];
    let input_b = vec![true; iterations as usize];

    let result = tandem::simulate(&program, &input_a, &input_b).unwrap();

    assert_eq!(result, vec![true]);

    Ok(())
}

fn xor(iterations: u32) -> Result<(), tandem::Error> {
    let mut gates = vec![Gate::InContrib];
    let output_gates = vec![iterations * 2];
    for i in 0..iterations {
        gates.append(&mut vec![Gate::InEval, Gate::And(i * 2, i * 2 + 1)]);
    }

    let program = Circuit::new(gates, output_gates);

    let input_a = vec![true];
    let input_b = vec![true; iterations as usize];

    let result = tandem::simulate(&program, &input_a, &input_b).unwrap();

    let expected = vec![iterations % 2 == 0];

    assert_eq!(result, expected);

    Ok(())
}

fn circuits_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("AND gates tandem");
    for iterations in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(iterations),
            iterations,
            |b, &iterations| {
                b.iter(|| and(iterations));
            },
        );
    }
    group.finish();

    let mut group = c.benchmark_group("XOR gates tandem");
    for iterations in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(iterations),
            iterations,
            |b, &iterations| {
                b.iter(|| xor(iterations));
            },
        );
    }
    group.finish();
}

criterion_group! {
  name = benches;
  config = Criterion::default();
  targets = circuits_benchmarks
}
criterion_main!(benches);
