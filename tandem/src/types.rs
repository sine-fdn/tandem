//! Common type definitions.

use std::ops::BitXor;

use rand::{CryptoRng, Rng, RngCore};
use serde::{Deserialize, Serialize};

/// The number bits of computational security.
pub(crate) const K: usize = SecurityBits::BITS as usize;

pub(crate) type SecurityBits = u128;

/// MAC data type underlying authenticated bits etc.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MacType(pub(crate) SecurityBits);

impl BitXor<u128> for MacType {
    type Output = MacType;

    fn bitxor(self, rhs: u128) -> Self::Output {
        MacType(self.0 ^ rhs)
    }
}

impl BitXor<MacType> for MacType {
    type Output = MacType;

    fn bitxor(self, rhs: MacType) -> Self::Output {
        MacType(self.0 ^ rhs.0)
    }
}

/// Data type of Keys underlying authenticated bits etc.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct KeyType(pub(crate) SecurityBits);

impl BitXor<u128> for KeyType {
    type Output = Self;

    fn bitxor(self, rhs: u128) -> Self::Output {
        Self(self.0 ^ rhs)
    }
}

impl BitXor<KeyType> for KeyType {
    type Output = Self;

    fn bitxor(self, rhs: KeyType) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl From<MacType> for KeyType {
    fn from(m: MacType) -> Self {
        Self(m.0)
    }
}

/// A wire mask generated during preprocessing. Foundation for garbled circuit computation.
#[derive(Default, Debug, Clone)]
pub(crate) struct WireMask {
    /// The wire label if {bit.bit} is `false`.
    pub(crate) label_0: WireLabel,
    /// The current mask which is used to hide the wire's actual value.
    pub(crate) bit: BitShare,
}

/// Evaluation state derived at function-dependant preprocessing stage.
#[derive(Default, Debug, Clone)]
pub(crate) struct WireState {
    /// The label for this wire, computed during preprocessing.
    pub(crate) label: WireLabel,
    /// The value of the wire after masking it with {bit.bit}.
    pub(crate) masked_value: bool,
    /// The AND table derived at preprocessing time, representing the local share.
    pub(crate) my_and_table: AndTableShare,
    /// The AND table from a contributing party, representing their share.
    pub(crate) other_and_table: AndTableShare,
}

pub(crate) type TableShare = (u32, [BitShare; 4]);
pub(crate) type InputMaskShare = (u32, PartialBitShare);

/// The share of a bit coming from F_Pre.
///
/// The mac relates to the bit {bit} while the {key} relates to the bit given to the other party.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BitShare {
    /// MAC key used for other party's bit.
    pub(crate) key: KeyType,
    /// MAC for this bit.
    pub(crate) mac: MacType,
    /// The actual bit i.e. value of this authenticated bit.
    pub(crate) bit: bool,
}

/// A partial bit share; used for disclosing one's authenticated bit.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PartialBitShare {
    /// The authenticated bit's MAC.
    pub(crate) mac: MacType,
    /// The authenticated bit's value.
    pub(crate) bit: bool,
}

/// Random bitmask used to construct AND gate table shares.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct WireLabel(pub(crate) SecurityBits);

/// The processing node-global hiding key AKA **THE DELTA**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delta(pub(crate) SecurityBits);

/// Share of an AND table.
pub(crate) type AndTableShare = [BitShare; 4];

impl Delta {
    pub(crate) fn gen_random<Rng: RngCore + CryptoRng>(rng: &mut Rng) -> Self {
        Self(rng.gen::<SecurityBits>())
    }

    #[inline]
    pub(crate) fn xor(&self, rhs: MacType) -> MacType {
        MacType(self.0 ^ rhs.0)
    }
}

impl From<SecurityBits> for WireLabel {
    fn from(label: SecurityBits) -> WireLabel {
        WireLabel(label)
    }
}

impl From<SecurityBits> for Delta {
    fn from(delta: SecurityBits) -> Delta {
        Delta(delta)
    }
}

impl From<BitShare> for PartialBitShare {
    fn from(b: BitShare) -> PartialBitShare {
        PartialBitShare::from(&b)
    }
}

impl From<&BitShare> for PartialBitShare {
    fn from(b: &BitShare) -> PartialBitShare {
        PartialBitShare {
            mac: b.mac,
            bit: b.bit,
        }
    }
}

impl WireLabel {
    fn delta_xor(&self, delta: &Delta) -> WireLabel {
        WireLabel(self.0 ^ delta.0)
    }

    pub(crate) fn xor(&self, rhs: &WireLabel) -> WireLabel {
        WireLabel(self.0 ^ rhs.0)
    }
}

impl WireMask {
    /// Returns a label matching the given {bit}.
    #[inline]
    pub(crate) fn label(&self, bit: bool, delta: &Delta) -> WireLabel {
        if bit {
            self.label_0.delta_xor(delta)
        } else {
            self.label_0.clone()
        }
    }

    /// Computes a new wire mask under XOR-homomorphism.
    #[inline]
    pub(crate) fn xor(&self, rhs: &WireMask) -> WireMask {
        let label_0 = self.label_0.xor(&rhs.label_0);
        WireMask {
            label_0,
            bit: self.bit.xor(&rhs.bit),
        }
    }

    /// Negates the current WireMask.
    ///
    /// (This is used to enable `NOT` gate support at the engine-level.)
    #[inline]
    pub(crate) fn not(&self, delta: &Delta) -> WireMask {
        WireMask {
            label_0: self.label_0.delta_xor(delta),
            bit: self.bit,
        }
    }
}

impl BitShare {
    /// XOR homomorphism.
    pub(crate) fn xor(&self, rhs: &BitShare) -> BitShare {
        BitShare {
            key: KeyType(self.key.0 ^ rhs.key.0),
            mac: MacType(self.mac.0 ^ rhs.mac.0),
            bit: self.bit ^ rhs.bit,
        }
    }
}

impl PartialBitShare {
    /// MAC verification of an authenticated bit.
    pub(crate) fn verify(&self, key: &KeyType, delta: &Delta) -> bool {
        (if self.bit { delta.0 } else { 0 }) ^ key.0 == self.mac.0
    }
}

#[test]
fn test_xor_impl() {
    for _ in 0..20 {
        let x: u128 = rand::random();
        let y: u128 = rand::random();

        assert_eq!(KeyType(x ^ y), KeyType(x) ^ KeyType(y));
        assert_eq!(MacType(x ^ y), MacType(x) ^ MacType(y));
        assert_eq!(KeyType(x ^ y), KeyType(x) ^ y);
        assert_eq!(MacType(x ^ y), MacType(x) ^ y);
    }
}
