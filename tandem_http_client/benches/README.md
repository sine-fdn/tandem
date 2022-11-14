# Tandem HTTP Client Benchmarks

This directory contains benchmarks for the Tandem engine, communicating over HTTP and running from the client's side.

All benchmark groups start their server based on configured handlers, providing the contributor's input for the
Tandem engine. The evaluator's input is also provided in the benchmark. Hence, benchmarks are self-contained and can be run with the usual command `cargo bench`.

All benchmarks were implemented with [Criterion.rs](https://github.com/bheisler/criterion.rs).

## `circuits.rs`

This file includes 4 benchmarks with only `AND` gates and 4 benchmarks with only `XOR` gates.

| Function    | Gates     |
| ----------- | --------- |
| `and_10`    | 10 `AND`  |
| `and_100`   | 100 `AND` |
| `and_1000`  | 1k `AND`  |
| `and_10000` | 10k `AND` |
| `xor_10`    | 10 `XOR`  |
| `xor_100`   | 100 `XOR` |
| `xor_1000`  | 1k `XOR`  |
| `xor_10000` | 10k `XOR` |

## `credit_scoring.rs`

This file contains a benchmark based on an integration test, simulating a real use case of the Tandem
engine. This example takes a credit scoring algorithm as the contributor's input and a user's data
and credit history as the evaluator's input. Keeping the algorithm and the user informations
private, the Tandem engine calculates the user's credit score.

| Function                   | Gates                           |
| -------------------------- | ------------------------------- |
| `credit_scoring_benchmark` | 24k `XOR`, 10k `NOT`, 33k `AND` |

## `multiplications.rs`

This file contains 2 benchmarks, with both `AND` and `XOR` gates, generated from simple Garble functions that multiply the contributor's input by the evaluator's input 1 and 10 times.

| Function | Gates                  |
| -------- | ---------------------- |
| `mul_1`  | 16k `AND`, 16k `XOR`   |
| `mul_10` | 160k `AND`, 161k `XOR` |
