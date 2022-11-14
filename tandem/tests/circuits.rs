use tandem::{Circuit, Error, Gate};

#[test]
fn test_missing_output_gates() -> Result<(), Error> {
    let program = Circuit::new(vec![Gate::InContrib, Gate::InEval, Gate::Xor(0, 1)], vec![]);

    for in_a in vec![true, false] {
        for in_b in vec![true, false] {
            let input_a = vec![in_a];
            let input_b = vec![in_b];

            let result = tandem::simulate(&program, &input_a, &input_b);

            assert_eq!(result, Err(Error::InvalidCircuit));
        }
    }

    Ok(())
}

#[test]
fn test_invalid_gates() -> Result<(), Error> {
    let invalid_xor = Circuit::new(
        vec![Gate::InContrib, Gate::InEval, Gate::Xor(0, 500)],
        vec![2],
    );
    let invalid_and = Circuit::new(
        vec![Gate::InContrib, Gate::InEval, Gate::And(0, 500)],
        vec![2],
    );
    let invalid_not = Circuit::new(vec![Gate::InContrib, Gate::InEval, Gate::Not(500)], vec![2]);

    for program in [invalid_xor, invalid_and, invalid_not] {
        for in_a in vec![true, false] {
            for in_b in vec![true, false] {
                let input_a = vec![in_a];
                let input_b = vec![in_b];

                let result = tandem::simulate(&program, &input_a, &input_b);
                assert_eq!(result, Err(Error::InvalidCircuit));
            }
        }
    }

    Ok(())
}

#[test]
fn test_invalid_output_gates() -> Result<(), Error> {
    let program = Circuit::new(
        vec![Gate::InContrib, Gate::InEval, Gate::Xor(0, 1)],
        vec![3],
    );

    for in_a in vec![true, false] {
        for in_b in vec![true, false] {
            let input_a = vec![in_a];
            let input_b = vec![in_b];

            let result = tandem::simulate(&program, &input_a, &input_b);

            assert_eq!(result, Err(Error::InvalidCircuit));
        }
    }

    Ok(())
}

#[test]
fn test_invalid_input_len() -> Result<(), Error> {
    let program = Circuit::new(
        vec![Gate::InContrib, Gate::InEval, Gate::Xor(0, 1)],
        vec![2],
    );

    for input_a in [vec![], vec![true], vec![true, false]] {
        for input_b in [vec![], vec![true], vec![true, false]] {
            let result = tandem::simulate(&program, &input_a, &input_b);

            if input_a.len() == 1 && input_b.len() == 1 {
                assert!(result.is_ok());
            } else {
                assert_eq!(result, Err(Error::InsufficientInput));
            }
        }
    }

    Ok(())
}

#[test]
fn test_max_gates_exceeded() -> Result<(), Error> {
    let max_gates = (u32::MAX >> 4) as usize;
    let mut gates = Vec::with_capacity(max_gates + 3);
    gates.push(Gate::InContrib);
    gates.push(Gate::InEval);
    for i in 0..max_gates + 1 {
        gates.push(Gate::Xor(0, i as u32))
    }
    let program = Circuit::new(gates, vec![2]);
    assert_eq!(program.validate(), Err(Error::MaxCircuitSizeExceeded));
    Ok(())
}

#[test]
fn test_max_and_gates_exceeded() -> Result<(), Error> {
    let max_and_gates = (u32::MAX >> 8) as usize;
    let mut gates = Vec::with_capacity(max_and_gates + 3);
    gates.push(Gate::InContrib);
    gates.push(Gate::InEval);
    for i in 0..max_and_gates + 1 {
        gates.push(Gate::And(0, i as u32))
    }
    let program = Circuit::new(gates, vec![2]);
    assert_eq!(program.validate(), Err(Error::MaxCircuitSizeExceeded));
    Ok(())
}

#[test]
fn test_xor() -> Result<(), Error> {
    let program = Circuit::new(
        vec![Gate::InContrib, Gate::InEval, Gate::Xor(0, 1)],
        vec![2],
    );

    for in_a in vec![true, false] {
        for in_b in vec![true, false] {
            let input_a = vec![in_a];
            let input_b = vec![in_b];

            let result = tandem::simulate(&program, &input_a, &input_b)?;

            assert_eq!(result, vec![in_a ^ in_b]);
        }
    }

    Ok(())
}

#[test]
fn test_not_simple() -> Result<(), Error> {
    let program = Circuit::new(
        vec![
            Gate::InContrib,
            Gate::InEval,
            Gate::Not(0),
            Gate::Not(1),
            Gate::Not(2),
            Gate::Not(3),
        ],
        vec![2, 3, 4, 5],
    );

    for in_a in vec![true, false] {
        for in_b in vec![true, false] {
            let input_a = vec![in_a];
            let input_b = vec![in_b];

            let result = tandem::simulate(&program, &input_a, &input_b)?;

            assert_eq!(result, vec![!in_a, !in_b, in_a, in_b]);
        }
    }
    Ok(())
}

#[test]
fn test_not() -> Result<(), Error> {
    let program = Circuit::new(
        vec![
            Gate::InContrib,
            Gate::InEval,
            Gate::Xor(0, 1),
            // gate 3 : !not(xor)
            Gate::Not(2),
            Gate::Not(0),
            Gate::Not(1),
            // gate 6: Xor(!a, b)
            Gate::Xor(4, 1),
            // gate 7: Xor(a, !b)
            Gate::Xor(0, 5),
            // gate 8: !Xor(a, !b)
            Gate::Not(7),
            Gate::And(0, 1),
            // gate 10: NAND(a, b)
            Gate::Not(9),
        ],
        vec![2, 3, 6, 7, 8, 10],
    );

    for in_a in vec![true, false] {
        for in_b in vec![true, false] {
            let input_a = vec![in_a];
            let input_b = vec![in_b];

            let result = tandem::simulate(&program, &input_a, &input_b)?;

            assert_eq!(
                result,
                vec![
                    in_a ^ in_b,
                    !(in_a ^ in_b),
                    (!in_a) ^ in_b,
                    in_a ^ (!in_b),
                    !(in_a ^ (!in_b)),
                    !(in_a & in_b)
                ]
            );
        }
    }

    Ok(())
}

#[test]
fn test_and_xor() -> Result<(), Error> {
    let program = Circuit::new(
        vec![
            Gate::InContrib,
            Gate::InEval,
            Gate::Xor(0, 1),
            Gate::And(1, 0),
        ],
        vec![2, 3],
    );

    for in_a in vec![true, false] {
        for in_b in vec![true, false] {
            let input_a = vec![in_a];
            let input_b = vec![in_b];

            let result = tandem::simulate(&program, &input_a, &input_b)?;

            assert_eq!(result, vec![in_a ^ in_b, in_a & in_b]);
        }
    }

    Ok(())
}

#[test]
fn test_and_deep() -> Result<(), Error> {
    let program = Circuit::new(
        vec![
            Gate::InContrib,
            Gate::InContrib,
            Gate::InEval,
            Gate::InEval,
            Gate::And(0, 2),
            Gate::And(1, 3),
            Gate::And(4, 5),
            Gate::Xor(4, 5),
        ],
        vec![4, 5, 6, 7],
    );

    for bitvec in 0..16 {
        let a0 = test_bit(bitvec, 0);
        let a1 = test_bit(bitvec, 1);
        let b0 = test_bit(bitvec, 2);
        let b1 = test_bit(bitvec, 3);

        let input_a = vec![a0, a1];
        let input_b = vec![b0, b1];

        let result = tandem::simulate(&program, &input_a, &input_b)?;

        assert_eq!(
            result,
            vec![a0 & b0, a1 & b1, a0 & b0 & a1 & b1, (a0 & b0) ^ (a1 & b1)],
            "a0={}, a1={}, b0={}, b1={}",
            a0,
            a1,
            b0,
            b1
        );
    }

    Ok(())
}

#[test]
fn test_large_and() -> Result<(), Error> {
    const N_AND_GATES: usize = 5;
    const N_RESULT_BITS: usize = N_AND_GATES;

    let mut and_gates: Vec<Gate> = (0..N_AND_GATES).map(|_| Gate::And(0, 1)).collect();
    let mut input_gates: Vec<Gate> = vec![Gate::InContrib, Gate::InEval];

    input_gates.append(&mut and_gates);

    let program = Circuit::new(input_gates, (2..(N_RESULT_BITS as u32 + 2)).collect());

    for bitvec in 3..4 {
        let a0 = test_bit(bitvec, 0);
        let b0 = test_bit(bitvec, 1);

        let input_a = vec![a0];
        let input_b = vec![b0];

        let result = tandem::simulate(&program, &input_a, &input_b)?;

        assert_eq!(
            result,
            Vec::from([a0 & b0; N_RESULT_BITS]),
            "Test: a0={:?} b0={:?}",
            a0,
            b0
        );
    }

    Ok(())
}

#[test]
fn test_insufficient_input() {
    let program = Circuit::new(
        vec![Gate::InContrib, Gate::InEval, Gate::Xor(0, 1)],
        vec![2],
    );

    assert_eq!(
        tandem::simulate(&program, &[], &[]),
        Err(Error::InsufficientInput)
    );
    assert_eq!(
        tandem::simulate(&program, &[true], &[]),
        Err(Error::InsufficientInput)
    );
    assert_eq!(
        tandem::simulate(&program, &[], &[true]),
        Err(Error::InsufficientInput)
    );
}

#[test]
fn test_unsupported_program() {
    let program = Circuit::new(vec![Gate::Xor(0, 0)], vec![0]);

    assert_eq!(
        tandem::simulate(&program, &[], &[]),
        Err(Error::InvalidCircuit)
    );
}

fn test_bit(value: i32, idx: u8) -> bool {
    (value & (1 << idx)) != 0
}
