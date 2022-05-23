pub use blake2b_rs::{Blake2b, Blake2bBuilder};

use ckb_types::{H160, H256};

pub const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";
pub const BLANK_HASH: [u8; 32] = [
    68, 244, 198, 151, 68, 213, 248, 197, 93, 100, 32, 98, 148, 157, 202, 228, 155, 196, 231, 239,
    67, 211, 136, 197, 161, 47, 66, 181, 99, 61, 22, 62,
];

pub fn new_blake2b() -> Blake2b {
    Blake2bBuilder::new(32)
        .personal(CKB_HASH_PERSONALIZATION)
        .build()
}

fn inner_blake2b_256<T: AsRef<[u8]>>(s: T) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut blake2b = new_blake2b();
    blake2b.update(s.as_ref());
    blake2b.finalize(&mut result);
    result
}

pub fn blake2b_256<T: AsRef<[u8]>>(s: T) -> [u8; 32] {
    if s.as_ref().is_empty() {
        return BLANK_HASH;
    }
    inner_blake2b_256(s)
}

pub fn blake2b_160<T: AsRef<[u8]>>(s: T) -> [u8; 20] {
    let mut result = [0u8; 20];
    let hash = blake2b_256(s);
    result.copy_from_slice(&hash[0..20]);
    result
}

pub fn blake2b_256_to_160(hash: &H256) -> H160 {
    let inner = hash.0;
    H160::from_slice(&inner[0..20]).expect("h256 to h160")
}
