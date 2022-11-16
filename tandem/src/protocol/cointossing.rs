//! Implements a simple coin tossing protocol.
//!
//! This protocol is used to allow 2 parties to generate the same randomly tossed coins / `Vec<u8>`,
//! which can then be used to seed the RNGs of both parties with the same seed.
//!
//! The protocol consists of:
//!   1. calling [`init`] to initialize the protocol, thereby disclosing the tuple item #2
//!      (commitment message) to the other party
//!   2. calling [`serialize`] on the return coin share (tuple item #1 from [`init`])
//!   3. finishing the protocol by calling [`finish`] with the other party's coin commitment and
//!      coin share messages
use crate::Error;

/// Number of bits for a coin.
pub(crate) const COIN_LEN: usize = 32;
/// Number of bits for a commitment.
const HASH_LEN: usize = blake3::OUT_LEN;

#[derive(Clone)]
pub(crate) struct CoinShare([u8; COIN_LEN]);

/// Result of the coin tossing protocol.
pub(crate) type CoinResult = [u8; COIN_LEN];

/// Creates a new coinshare and a message to be shared with another party.
pub(crate) fn init(coin: [u8; COIN_LEN]) -> Result<(CoinShare, Vec<u8>), Error> {
    let hash = hash_coinshare(&coin);
    let msg = bincode::serialize(&hash)?;
    let coin_share = CoinShare(coin);
    Ok((coin_share, msg))
}

/// Serializes a CoinShare to be disclosed to another party at the 2nd protocol step.
pub(crate) fn serialize(cs: &CoinShare) -> Result<Vec<u8>, Error> {
    let msg = bincode::serialize(&cs.0)?;
    Ok(msg)
}

/// Verifies the upstream coinshare and returns the resulting coin.
pub(crate) fn finish(
    coin_share: CoinShare,
    upstream_hash_msg: Vec<u8>,
    upstream_coin: Vec<u8>,
) -> Result<CoinResult, Error> {
    let upstream_hash: [u8; HASH_LEN] = bincode::deserialize(&upstream_hash_msg)?;
    let upstream_coin: [u8; COIN_LEN] = bincode::deserialize(&upstream_coin)?;

    if upstream_hash != hash_coinshare(&upstream_coin) {
        return Err(Error::MacError);
    }

    Ok(xor(coin_share.0, upstream_coin))
}

fn hash_coinshare(s: &[u8; COIN_LEN]) -> [u8; HASH_LEN] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(s);
    let mut output_reader = hasher.finalize_xof();
    let mut result = [0u8; HASH_LEN];
    output_reader.fill(&mut result);

    result
}

fn xor(lhs: [u8; COIN_LEN], rhs: [u8; COIN_LEN]) -> CoinResult {
    let mut result = [0u8; COIN_LEN];
    for i in 0..COIN_LEN {
        result[i] = lhs[i] ^ rhs[i];
    }

    result
}

#[test]
fn test_coinshare() {
    use rand::RngCore;
    use rand::SeedableRng;

    let test_val = (rand_chacha::ChaCha20Rng::from_entropy().next_u32() % 255) as u8;
    let coin1 = [test_val; COIN_LEN];
    let coin2 = [!test_val; COIN_LEN];
    let expected = [255u8; COIN_LEN];

    let (coin_share1, commitment_msg1) = init(coin1).unwrap();
    let coin_msg1 = serialize(&coin_share1).unwrap();

    let (coin_share2, commitment_msg2) = init(coin2).unwrap();
    let coin_msg2 = serialize(&coin_share2).unwrap();

    assert_eq!(
        expected,
        finish(coin_share1, commitment_msg2, coin_msg2).unwrap()
    );
    assert_eq!(
        expected,
        finish(coin_share2, commitment_msg1, coin_msg1).unwrap()
    );
}

#[test]
fn test_coinshare_fail() {
    use rand::Rng;
    use rand::SeedableRng;
    let mut rng = rand_chacha::ChaCha20Rng::from_entropy();
    let mut coin1: [u8; COIN_LEN] = Default::default();
    let mut coin2: [u8; COIN_LEN] = Default::default();
    rng.fill(&mut coin1[0..]);
    rng.fill(&mut coin2[0..]);
    let coin1 = coin1;
    let coin2 = coin2;

    let corruption_index = rng.gen_range(0..COIN_LEN * 8);

    let (coin_share1, _) = init(coin1).unwrap();
    let (coin_share2_ok, commitment_msg2_ok) = init(coin2.clone()).unwrap();
    let coin_msg2_ok = serialize(&coin_share2_ok).unwrap();

    // randomly corrupt the coin value by 1 bit and check that the protocol fails
    {
        let mut coin2 = coin2.clone();
        coin2[corruption_index / 8] ^= 1 << (corruption_index % 8);

        let (coin_share2_nok, commitment_msg2_nok) = init(coin2).unwrap();
        let coin_msg2_nok = serialize(&coin_share2_nok).unwrap();

        assert_eq!(
            Err(Error::MacError),
            finish(
                coin_share1.clone(),
                commitment_msg2_nok,
                coin_msg2_ok.clone()
            )
        );

        assert_eq!(
            Err(Error::MacError),
            finish(
                coin_share1.clone(),
                commitment_msg2_ok,
                coin_msg2_nok.clone()
            )
        );
    }
}
