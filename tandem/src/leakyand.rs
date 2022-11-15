//! WRK17 sub-protocols `Π_{HaAND}` and `Π_{LaAND}`.
use crate::{
    hash::hash,
    types::{Delta, KeyType, MacType, K},
};

pub(crate) type AndHashes = [[MacType; 2]];

/// Implements steps 1-3 of `Π_{HaAND}`; i.e. it creates [`K`]-many hashes for each secret value to
/// be sent to the other party.
///
/// - the parameters `authenticated_bits_y`, `keys` are the result of a leakydelta_ot exchange; i.e.
///   `keys[i]` is the key for authenticated bit at index `i`
/// - parameter `random_bits` equals variable `t_i` (resp. `s_i`) of the `Π_{HaAND}` protocol
/// - delta is the local delta...
///
/// i.e. this is a "vectorized" implementation of the protocol to operate on batches of `N` many
/// authenticated bits at once.
pub(crate) fn compute_leaky_and_hashes(
    out: &mut AndHashes,
    delta: &Delta,
    random_bits: u128,
    authenticated_bits_y: u128,
    keys: &[KeyType],
) {
    assert!(keys.len() >= K);
    assert!(out.len() >= K);

    for i in 0..K {
        let random_bit = u128::from(random_bits & (1 << i) != 0);
        let y_bit = u128::from((authenticated_bits_y & (1 << i)) != 0);
        out[i][0] = MacType(hash(MacType(keys[i].0)).0 ^ random_bit);
        out[i][1] = MacType(hash(delta.xor(MacType(keys[i].0))).0 ^ random_bit ^ y_bit);
    }
}

/// Implements the 2nd part of `Π_{HaAND}` of step 3 plus step 4.
///
/// It takes K-many `and_hashes` from the other party which were computed through
/// [compute_leaky_and_hashes]. This function outputs K-many `v_i` as per the `Π_{HaAND}` protocol.
pub(crate) fn derive_and_shares(
    random_bits: u128,
    authenticated_bits: u128,
    macs: &[MacType],
    and_hashes: &AndHashes,
) -> MacType {
    assert!(macs.len() >= K);
    assert!(and_hashes.len() >= K);

    let mut result = 0;

    for i in 0..K {
        let idx = usize::from((authenticated_bits & (1 << i)) != 0);
        let is_set = (and_hashes[i][idx].0 ^ hash(macs[i]).0) != 0;
        result |= (u128::from(is_set)) << i;
    }

    MacType(result ^ random_bits)
}

#[test]
fn test_leaky_and_hashes() {
    use rand::random;

    let (d_a, bits_a, keys_a, macs_a, d_b, bits_b, keys_b, macs_b) = gen_abits();
    let random_y1 = random();
    let random_y2 = random();

    let mut hashes_a: [[MacType; 2]; 128] = [[MacType(0), MacType(0)]; 128];
    let mut hashes_b: [[MacType; 2]; 128] = [[MacType(0), MacType(0)]; 128];

    let random_a = random();
    let random_b = random();
    compute_leaky_and_hashes(&mut hashes_b, &d_a, random_a, random_y1, &keys_a);
    compute_leaky_and_hashes(&mut hashes_a, &d_b, random_b, random_y2, &keys_b);

    let shares_a = derive_and_shares(random_a, bits_a.0, &macs_a, &hashes_a);
    let shares_b = derive_and_shares(random_b, bits_b.0, &macs_b, &hashes_b);

    assert_eq!(
        (shares_a.0 ^ shares_b.0),
        ((bits_a.0 & random_y2) ^ (bits_b.0 & random_y1))
    );
}

/// Generates [`K`]-many authenticated bits.
#[cfg(test)]
fn gen_abits() -> (
    Delta,
    KeyType,
    [KeyType; 128],
    [MacType; 128],
    Delta,
    KeyType,
    [KeyType; 128],
    [MacType; 128],
) {
    use rand::{random, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::from_entropy();
    let delta_a = Delta::gen_random(&mut rng);
    let delta_b: Delta = Delta::gen_random(&mut rng);

    let bits_a = KeyType(random());
    let bits_b = KeyType(random());

    let mut keys_a = [KeyType(0); 128];
    for k in keys_a.iter_mut() {
        *k = KeyType(random());
    }
    let mut keys_b = [KeyType(0); 128];
    for k in keys_b.iter_mut() {
        *k = KeyType(random());
    }
    let mut macs_a = [MacType(0); 128];
    for (i, m) in macs_a.iter_mut().enumerate() {
        *m = if bits_a.0 & 1 << i != 0 {
            delta_b.xor(MacType(keys_b[i].0))
        } else {
            MacType(keys_b[i].0)
        };
    }
    let mut macs_b = [MacType(0); 128];
    for (i, m) in macs_b.iter_mut().enumerate() {
        *m = if bits_b.0 & 1 << i != 0 {
            delta_a.xor(MacType(keys_a[i].0))
        } else {
            MacType(keys_a[i].0)
        };
    }
    (
        delta_a, bits_a, keys_a, macs_a, delta_b, bits_b, keys_b, macs_b,
    )
}
