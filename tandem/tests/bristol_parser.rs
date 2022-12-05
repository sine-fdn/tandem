use tandem::{simulate, Circuit};

#[test]
fn adder_64() -> () {
    let bristol_64_adder = include_str!("adder64.txt");

    let circuit_from_bristol = Circuit::from_bristol_format(bristol_64_adder);

    // println!("{:?}", circuit_from_bristol);

    let contrib_in = 42u64;

    let eval_in = 5u64;

    let contrib_bits = format!("{contrib_in:064b}");

    let eval_bits = format!("{eval_in:064b}");

    let mut contrib_bools: Vec<bool> = vec![];

    for bit in contrib_bits.chars() {
        if bit == '0' {
            contrib_bools.push(false);
        } else {
            contrib_bools.push(true);
        }
    }

    let mut eval_bools: Vec<bool> = vec![];

    for bit in eval_bits.chars() {
        if bit == '0' {
            eval_bools.push(false);
        } else {
            eval_bools.push(true);
        }
    }

    // println!("{:?}", contrib_bools);
    println!("{:?}", circuit_from_bristol.unwrap());
    // println!("{:?}", circuit_from_bristol.unwrap().validate());

    // let result = simulate(&circuit_from_bristol.unwrap(), &contrib_bools, &eval_bools);

    // let expected = 47u64;

    // let expected_bits = format!("{expected:064b}");

    // let mut expected_bools: Vec<bool> = vec![];

    // for bit in expected_bits.chars() {
    //     if bit == '0' {
    //         expected_bools.push(false);
    //     } else {
    //         expected_bools.push(true);
    //     }
    // }


    // assert_eq!(expected_bools, result.unwrap_or_else(|e| panic!("{e}")))
}
