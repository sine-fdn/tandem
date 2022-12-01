use tandem::Circuit;
use tandem_garble_interop::{check_program, compile_program};

#[test]
fn parse_bristol() -> () {
    let garble_64_adder = "pub fn adder_64(a: u64, b: u64) -> u64 {
        a + b
    }";

    let garble_program = check_program(garble_64_adder).unwrap();

    let compile_from_garble = compile_program(&garble_program, "adder_64").unwrap();

    let circuit_from_garble = compile_from_garble.gates;

    let bristol_64_adder = include_str!("aes128.txt");

    let circuit_from_bristol = Circuit::from_bristol_format(bristol_64_adder);

    assert_eq!(circuit_from_garble, circuit_from_bristol);
}
