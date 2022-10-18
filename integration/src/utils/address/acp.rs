use crate::const_definition::ANYONE_CAN_PAY_DEVNET_TYPE_HASH;

use anyhow::Result;
use ckb_types::{core::ScriptHashType, packed, prelude::*};
use common::{Address, AddressPayload, NetworkType};

pub fn build_acp_address(secp_address: &Address) -> Result<Address> {
    let secp_script: packed::Script = secp_address.payload().into();
    let anyone_can_pay_script = packed::ScriptBuilder::default()
        .code_hash(ANYONE_CAN_PAY_DEVNET_TYPE_HASH.pack())
        .args(secp_script.args())
        .hash_type(ScriptHashType::Type.into())
        .build();
    let payload = AddressPayload::from_script(&anyone_can_pay_script);
    Ok(Address::new(NetworkType::Dev, payload, true))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::const_definition::SIGHASH_TYPE_HASH;
    use crate::utils::address::generate_rand_secp_address_pk_pair;
    use crate::utils::address::is_secp256k1;
    use std::str::FromStr;

    #[test]
    fn test_build_addresses() {
        let _ = common::lazy::SECP256K1_CODE_HASH.set(SIGHASH_TYPE_HASH);

        let (address, _) = generate_rand_secp_address_pk_pair();
        assert!(is_secp256k1(&address));

        let address_secp =
            Address::from_str("ckt1qyqf4n4g6qfrvnp78ry4sm0tn8wgpjqf6ufq74srld").unwrap();
        let acp_address = build_acp_address(&address_secp).unwrap();
        assert_eq!("ckt1qp3g8fre50846snkekf4jn0f7xp84wd4t3astv7j3exzuznfdnl06qv6e65dqy3kfslr3j2cdh4enhyqeqyawysf7sf4c".to_string(), acp_address.to_string());
    }
}
