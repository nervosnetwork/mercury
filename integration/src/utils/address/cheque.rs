use crate::const_definition::CHEQUE_DEVNET_TYPE_HASH;

use anyhow::{anyhow, Result};
use ckb_types::{core::ScriptHashType, packed, prelude::*};
use common::{address::is_secp256k1, hash::blake2b_160, Address, AddressPayload, NetworkType};

pub fn build_cheque_address(
    receiver_address: &Address,
    sender_address: &Address,
) -> Result<Address> {
    if !is_secp256k1(receiver_address) || !is_secp256k1(sender_address) {
        return Err(anyhow!("can't get cheque address"));
    }
    let receiver_script: packed::Script = receiver_address.payload().into();
    let sender_script: packed::Script = sender_address.payload().into();
    let mut args = blake2b_160(receiver_script.as_slice()).to_vec();
    let sender = blake2b_160(sender_script.as_slice());
    args.extend_from_slice(&sender);
    let sudt_type_script = packed::ScriptBuilder::default()
        .code_hash(CHEQUE_DEVNET_TYPE_HASH.pack())
        .args(args.pack())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&sudt_type_script);
    Ok(Address::new(NetworkType::Dev, payload, true))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::const_definition::SIGHASH_TYPE_HASH;
    use crate::utils::address::generate_rand_secp_address_pk_pair;
    use std::str::FromStr;

    #[test]
    fn test_build_addresses() {
        let _ = common::lazy::SECP256K1_CODE_HASH.set(SIGHASH_TYPE_HASH);

        let (address, _) = generate_rand_secp_address_pk_pair();
        assert!(is_secp256k1(&address));

        let sender = Address::from_str("ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06y24q4tc4tfkgze35cc23yprtpzfrzygljdjh9").unwrap();
        let receiver = Address::from_str("ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld").unwrap();
        let cheque = build_cheque_address(&receiver, &sender).unwrap();
        assert_eq!("ckt1qqdpunl0xn6es2gx7azmqj870vggjer7sg6xqa8q7vkzan3xea43uqt6g2dxvxxjtdhfvfs0f67gwzgrcrfg3gj9yywse6zu05ez3s64xmtdkl6074rac6q3f7cvk".to_string(), cheque.to_string());
    }
}
