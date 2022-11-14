pub fn and_10(a: bool, b: [bool; 10]) -> bool {
    let mut acc_and = a;
    for i in b {
        acc_and = acc_and & i
    }
    acc_and
}

pub fn and_100(a: bool, b: [bool; 100]) -> bool {
    let mut acc_and = a;
    for i in b {
        acc_and = acc_and & i
    }
    acc_and
}

pub fn and_1000(a: bool, b: [bool; 1000]) -> bool {
    let mut acc_and = a;
    for i in b {
        acc_and = acc_and & i
    }
    acc_and
}

pub fn and_10000(a: bool, b: [bool; 10000]) -> bool {
    let mut acc_and = a;
    for i in b {
        acc_and = acc_and & i
    }
    acc_and
}

pub fn xor_10(a: bool, b: [bool; 10]) -> bool {
    let mut acc_xor = a;
    for i in b {
        acc_xor = acc_xor ^ i
    }
    acc_xor
}

pub fn xor_100(a: bool, b: [bool; 100]) -> bool {
    let mut acc_xor = a;
    for i in b {
        acc_xor = acc_xor ^ i
    }
    acc_xor
}

pub fn xor_1000(a: bool, b: [bool; 1000]) -> bool {
    let mut acc_xor = a;
    for i in b {
        acc_xor = acc_xor ^ i
    }
    acc_xor
}

pub fn xor_10000(a: bool, b: [bool; 10000]) -> bool {
    let mut acc_xor = a;
    for i in b {
        acc_xor = acc_xor ^ i
    }
    acc_xor
}
