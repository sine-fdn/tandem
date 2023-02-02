//! Chou Orlandi Simplest OT protocol based on a version from [ABKLX21].
//!
//!
//! [ABKLX21]: https://eprint.iacr.org/2021/1218.pdf
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;

pub(crate) const MSG_LEN: usize = 32;

/// The type of (random) message exchanged via the Base OT Protocol.
pub(crate) type OtMessage = [u8; MSG_LEN];

/// The party sending data to a [`Receiver`].
///
/// I.e. the logical actor offering 2 pieces of data of which the [`Receiver`] will be able to
/// recover only 1.
#[derive(Clone)]
pub(crate) struct Sender {
    private_key: Scalar,
    pub_key: RistrettoPoint,
    pub_key_squared: RistrettoPoint,
}

/// The party choosing 1-out-of-2 pieces of data w/o the [`Sender`] knowing which it was.
#[derive(Clone)]
pub(crate) struct Receiver {
    private_key: Scalar,
    upstream_pub_key: RistrettoPoint,
    choice: bool,
}

/// The kind of messages exchanged between a [`Sender`] and a [`Receiver`].
pub(crate) mod message {
    use std::slice;

    use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};

    use crate::Error;

    use super::OtMessage;

    /// Message to initiate the protocol; sent between [`super::Sender`] and [`super::Receiver`] at
    /// first OT protocol step.
    #[derive(Debug, Copy, Clone, Default, PartialEq)]
    pub(crate) struct Init(pub(crate) RistrettoPoint);

    /// Reply to the [`Init`] message, sent by the [`super::Receiver`] to the [`super::Sender`].
    #[derive(Default, Debug, Clone, Copy, PartialEq)]
    pub(crate) struct InitReply(pub(super) [OtMessage; 2]);

    impl Init {
        pub(crate) fn serialize_to_buffer(&self, buffer: &mut Vec<u8>) {
            buffer.extend(self.0.compress().as_bytes());
        }

        pub(crate) fn deserialize_from_buffer(buffer: &mut slice::Iter<u8>) -> Result<Self, Error> {
            let key_bytes = read_key_from_buffer(buffer)?;
            let point = CompressedRistretto(key_bytes)
                .decompress()
                .ok_or(Error::OtInitDeserializationError)?;

            Ok(Self(point))
        }
    }

    impl InitReply {
        pub(crate) fn serialize_to_buffer(&self, buffer: &mut Vec<u8>) {
            buffer.extend(self.0[0]);
            buffer.extend(self.0[1]);
        }

        pub(crate) fn deserialize_from_buffer(buffer: &mut slice::Iter<u8>) -> Result<Self, Error> {
            let blinding_keys = [read_key_from_buffer(buffer)?, read_key_from_buffer(buffer)?];
            Ok(Self(blinding_keys))
        }
    }

    fn read_key_from_buffer(buffer: &mut slice::Iter<u8>) -> Result<OtMessage, Error> {
        let mut key = [0u8; super::MSG_LEN];
        for b in &mut key {
            *b = *buffer.next().ok_or(Error::OtInitDeserializationError)?;
        }
        Ok(key)
    }
}

impl Sender {
    /// Creates a new `OT*` protocol sender.
    pub(crate) fn new<RNG>(rng: &mut RNG) -> Self
    where
        RNG: rand::RngCore + rand::CryptoRng,
    {
        let private_key = Scalar::random::<RNG>(rng);
        let pub_key = RISTRETTO_BASEPOINT_TABLE * &private_key;
        let pub_key_squared = pub_key * private_key;

        Self {
            private_key,
            pub_key,
            pub_key_squared,
        }
    }

    /// Creates an [`message::Init`] message suitable for exchange with a [`Receiver`].
    ///
    /// Implements step 2 of protocol `OT*`.
    pub(crate) fn init_message(&self) -> message::Init {
        message::Init(self.pub_key)
    }

    /// The logical "send" part of the CO protocol.
    ///
    /// Sends the 2 OT messages to the [`Receiver`] via the message [`message::InitReply`].
    ///
    /// Implements step 3 of the protocol:
    ///
    /// Computation of `k_b := H(A, U^y B^{−b})` with
    ///   - `A := self.pub_key`
    ///   - `U := upstream_init.0`
    ///   - `B := self.pub_key_squared`
    ///   - `b := choice bit [false, true]`
    pub(crate) fn send(
        &self,
        upstream_init: &message::Init,
        messages: &[OtMessage; 2],
    ) -> message::InitReply {
        let upstream_pub_key = upstream_init.0;
        let my_pub_key_bytes = self.pub_key.compress().to_bytes();

        let mut hasher = blake3::Hasher::new();

        // e_0 = H(A, U^y) XOR m_0
        let key0 = {
            hasher.update(&my_pub_key_bytes);
            let upstream_bytes = (upstream_pub_key * self.private_key).compress().to_bytes();
            hasher.update(&upstream_bytes);
            let hash = hasher.finalize();
            Self::xor_keys(hash.as_bytes(), &messages[0])
        };
        hasher.reset();

        // e_1 = H(A, U^y B^{−b}) XOR m_1
        let key1 = {
            hasher.update(&my_pub_key_bytes);
            let upstream_bytes = ((upstream_pub_key * self.private_key) - self.pub_key_squared)
                .compress()
                .to_bytes();
            hasher.update(&upstream_bytes);
            let hash = hasher.finalize();
            Self::xor_keys(hash.as_bytes(), &messages[1])
        };

        message::InitReply([key0, key1])
    }

    #[inline]
    fn xor_keys(lhs: &OtMessage, rhs: &OtMessage) -> OtMessage {
        let mut result = [0u8; MSG_LEN];
        for idx in 0..MSG_LEN {
            result[idx] = lhs[idx] ^ rhs[idx];
        }
        result
    }
}

impl Receiver {
    /// Base OT Receiving function.
    ///
    /// Implements step 2 of protocol CO (Figure 4) of ABKLX21.
    pub(crate) fn init<RNG>(
        rng: &mut RNG,
        upstream_init: &message::Init,
        choice: bool,
    ) -> (message::Init, Receiver)
    where
        RNG: rand::RngCore + rand::CryptoRng,
    {
        let private_key = Scalar::random(rng);

        let upstream_pub_key = upstream_init.0;
        let my_pub_key = RISTRETTO_BASEPOINT_TABLE * &private_key;

        let chosen_pub_key = {
            let choices = [my_pub_key, upstream_pub_key + my_pub_key];
            choices[usize::from(choice)]
        };

        let init_msg = message::Init(chosen_pub_key);
        let receiver = Receiver {
            private_key,
            upstream_pub_key,
            choice,
        };

        (init_msg, receiver)
    }

    /// The logical "receive" part of the CO protocol.
    ///
    /// Implements step 4 of the CO protocol (Figure 4) of ABKLX21:
    ///
    /// Computation of
    ///   1. `"k_b := H(A, A^x)"`
    ///   2. `"m_b := e_b XOR k_b"`
    ///
    ///  with
    ///   - `A := self.upstream_pub_key`
    ///   - `x := self.private_key`
    ///   - `m_b := message.0[self.choice]`
    pub(crate) fn recv(self, upstream_init_reply: message::InitReply) -> OtMessage {
        // step 1 from above
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.upstream_pub_key.compress().to_bytes());
        hasher.update(
            &(self.upstream_pub_key * self.private_key)
                .compress()
                .to_bytes(),
        );
        let hash = hasher.finalize();

        // step 2 of above
        let upstream_blinding_keys = upstream_init_reply.0;
        let blinding_key = upstream_blinding_keys[self.choice as usize];
        let mut result = [0u8; MSG_LEN];
        for idx in 0..MSG_LEN {
            result[idx] = hash.as_bytes()[idx] ^ blinding_key[idx];
        }

        result
    }
}

#[test]
fn test_abklx21() {
    use rand::RngCore;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng_send = ChaCha20Rng::from_entropy();
    let mut rng_recv = ChaCha20Rng::from_entropy();

    for choice in [false, true] {
        let mut messages = [OtMessage::default(); 2];
        rng_send.fill_bytes(&mut messages[0]);
        rng_send.fill_bytes(&mut messages[1]);

        let s = Sender::new(&mut rng_send);
        let init = Sender::init_message(&s);
        let (msg, r) = Receiver::init(&mut rng_recv, &init, choice);
        let reply = s.send(&msg, &messages);

        let key = r.recv(reply);

        assert_eq!(key, messages[choice as usize]);
        assert_ne!(key, messages[if choice { 0 } else { 1 }]);
    }
}
