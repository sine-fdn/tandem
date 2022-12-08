use tandem::{simulate, Circuit};

#[test]
fn adder_64() -> () {
    let bristol_adder_64 = include_str!("adder64.txt");

    let circuit = Circuit::from_bristol_format(bristol_adder_64).unwrap();

    let contrib_in = 4u64;
    let contrib_bits = format!("{contrib_in:064b}");

    let mut contrib_tandem: Vec<bool> = vec![];

    for bit in contrib_bits.chars() {
        if bit == '0' {
            contrib_tandem.push(false);
        } else {
            contrib_tandem.push(true);
        }
    }

    let eval_in = 5u64;
    let eval_bits = format!("{eval_in:064b}");

    let mut eval_tandem: Vec<bool> = vec![];

    for bit in eval_bits.chars() {
        if bit == '0' {
            eval_tandem.push(false);
        } else {
            eval_tandem.push(true);
        }
    }

    let result = simulate(
        &circuit,
        &contrib_tandem.into_iter().rev().collect::<Vec<_>>(),
        &eval_tandem.into_iter().rev().collect::<Vec<_>>(),
    )
    .unwrap()
    .into_iter()
    .rev()
    .collect::<Vec<_>>();

    let expected = 9u64;

    let expected_bits = format!("{expected:064b}");

    let mut expected_tandem: Vec<bool> = vec![];

    for bit in expected_bits.chars() {
        if bit == '0' {
            expected_tandem.push(false);
        } else {
            expected_tandem.push(true);
        }
    }

    assert_eq!(expected_tandem, result)
}

#[test]
fn aes_128() -> () {
    let bristol_aes_128 = include_str!("aes128.txt");

    let circuit = Circuit::from_bristol_format(bristol_aes_128).unwrap();

    let key_hex = "2b7e151628aed2a6abf7158809cf4f3c";

    let key_num = u128::from_str_radix(key_hex, 16).unwrap();

    let key_bin = format!("{key_num:0128b}");

    let mut key_tandem: Vec<bool> = vec![];

    for bit in key_bin.chars() {
        if bit == '0' {
            key_tandem.push(false);
        } else {
            key_tandem.push(true);
        }
    }

    let plain_hex = "f69f2445df4f9b17ad2b417be66c3710";

    let plain_num = u128::from_str_radix(plain_hex, 16).unwrap();

    let plain_bin = format!("{plain_num:0128b}");

    let mut plain_tandem: Vec<bool> = vec![];

    for bit in plain_bin.chars() {
        if bit == '0' {
            plain_tandem.push(false);
        } else {
            plain_tandem.push(true);
        }
    }

    let result = simulate(
        &circuit,
        &key_tandem.into_iter().rev().collect::<Vec<_>>(),
        &plain_tandem.into_iter().rev().collect::<Vec<_>>(),
    )
    .unwrap()
    .into_iter()
    .rev()
    .collect::<Vec<_>>();

    let cipher_hex = "7b0c785e27e8ad3f8223207104725dd4";

    let cipher_num = u128::from_str_radix(cipher_hex, 16).unwrap();

    let cipher_bin = format!("{cipher_num:0128b}");

    let mut cipher_tandem: Vec<bool> = vec![];

    for bit in cipher_bin.chars() {
        if bit == '0' {
            cipher_tandem.push(false);
        } else {
            cipher_tandem.push(true);
        }
    }

    assert_eq!(result, cipher_tandem);
}

#[test]
fn sha_256() -> () {
    let bristol_sha_256 = include_str!("sha256.txt");
    let circuit = Circuit::from_bristol_format(bristol_sha_256).unwrap();

    let message = ["2b7e151628aed2a6abf7158809cf4f3c"; 4];

    let mut message_bin = vec![];

    for i in 0..4 {
        let n = u128::from_str_radix(message[i], 16).unwrap();
        let n_bin = format!("{n:0128b}");
        message_bin.push(n_bin);
    }

    let message_bin = message_bin.join("");

    let mut message_tandem = vec![];

    for bit in message_bin.chars() {
        if bit == '0' {
            message_tandem.push(false);
        } else {
            message_tandem.push(true);
        }
    }

    let chaining_value = [0u128; 2];

    let mut chaining_bin = vec![];

    for i in 0..2 {
        let n = chaining_value[i];
        let bin = format!("{n:0128b}");
        chaining_bin.push(bin);
    }

    let chaining_bin = chaining_bin.join("");

    let mut chaining_tandem = vec![];

    for bit in chaining_bin.chars() {
        if bit == '0' {
            chaining_tandem.push(false);
        } else {
            chaining_tandem.push(true);
        }
    }

    let result = simulate(&circuit, &message_tandem, &chaining_tandem).unwrap();

    println!("{result:?}");
}
