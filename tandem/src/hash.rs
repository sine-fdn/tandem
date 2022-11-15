//! WRK17-compatible non-optimized hashing based on [`blake3::Hasher`].
use blake3::OutputReader;

use crate::types::{KeyType, MacType};

/// WRK17-specific hashing for building garbled tables.
pub(crate) mod garbling_hash {
    use crate::{
        types::{BitShare, KeyType, MacType, WireLabel},
        GateIndex,
    };
    use std::io::Read;

    /// Computes a garbled table share.
    pub(crate) fn new(
        label_x: &WireLabel,
        label_y: &WireLabel,
        gate: GateIndex,
        row: u8,
    ) -> BitShare {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&label_x.0.to_le_bytes());
        hasher.update(&label_y.0.to_le_bytes());
        hasher.update(&gate.to_le_bytes());
        hasher.update(&[row]);
        let mut output_reader = hasher.finalize_xof();

        let mut mac_buffer: [u8; 16] = [0; 16];
        let mut label_buffer: [u8; 16] = [0; 16];
        let mut bit_buffer: [u8; 1] = [0];

        let r = output_reader.read(&mut mac_buffer);
        assert!(r.is_ok());
        let r = output_reader.read(&mut label_buffer);
        assert!(r.is_ok());
        let r = output_reader.read(&mut bit_buffer);
        assert!(r.is_ok());

        assert_ne!(mac_buffer, [0; 16]);
        assert_ne!(label_buffer, [0; 16]);

        BitShare {
            mac: MacType(u128::from_le_bytes(mac_buffer)),
            key: KeyType(u128::from_le_bytes(label_buffer)),
            bit: (bit_buffer[0] & 1) == 1,
        }
    }

    #[test]
    fn test_new() {
        let h0 = new(&WireLabel(0), &WireLabel(1), 0, 0);
        let h1 = new(&WireLabel(0), &WireLabel(1), 0, 1);
        assert_ne!(h0, h1);
    }
}

pub(crate) fn hash(mac: MacType) -> MacType {
    hash_u128(mac.0)
}

pub(crate) fn hash_key(key: KeyType) -> MacType {
    hash_u128(key.0)
}

fn hash_u128(key: u128) -> MacType {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&key.to_le_bytes());

    let output_reader = hasher.finalize_xof();
    MacType(blake3_to_u128(output_reader))
}

pub(crate) fn hash_keys(k1: KeyType, k2: KeyType) -> MacType {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&k1.0.to_le_bytes());
    hasher.update(&k2.0.to_le_bytes());

    let output_reader = hasher.finalize_xof();
    MacType(blake3_to_u128(output_reader))
}

#[inline]
fn blake3_to_u128(mut output_reader: OutputReader) -> u128 {
    let mut result = [0u8; 16];
    output_reader.fill(&mut result);
    u128::from_le_bytes(result)
}

#[test]
fn reference_hash_values() {
    let r0 = 164479851121213158701332959497568687214_u128;
    let r1 = 32869993993155099816536977414117934351_u128;

    assert_eq!(252301825721988224801639279640745335827, hash(MacType(r0)).0);
    assert_eq!(19881579897213927600698344798095172587, hash(MacType(r1)).0);
    assert_eq!(
        265242760764573362325515364989468422452,
        hash_keys(KeyType(r0), KeyType(r1)).0
    );
}

#[test]
fn randomized_hashing() {
    let r: u128 = rand::random();
    let ref_0 = blake3::hash(&r.to_le_bytes());
    assert_eq!(&ref_0.as_bytes()[..16], hash(MacType(r)).0.to_ne_bytes());
}
