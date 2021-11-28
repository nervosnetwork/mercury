use crate::{
    NetworkType, ACP_MAINNET_TYPE_HASH, ACP_TESTNET_TYPE_HASH, MULTISIG_TYPE_HASH,
    SIGHASH_TYPE_HASH,
};

use bech32::{convert_bits, ToBase32, Variant};
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, packed, prelude::*, H160, H256};
use serde::{Deserialize, Serialize};

use std::convert::TryInto;
use std::fmt;
use std::str::FromStr;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum AddressType {
    // full version identifies the hash_type and vm_version
    Full = 0x00,
    // short version for locks with popular code_hash
    Short = 0x01,
    // full version with hash_type = "Data", deprecated
    FullData = 0x02,
    // full version with hash_type = "Type", deprecated
    FullType = 0x04,
}

impl AddressType {
    pub fn from_u8(value: u8) -> Result<AddressType, String> {
        match value {
            0x00 => Ok(AddressType::Full),
            0x01 => Ok(AddressType::Short),
            0x02 => Ok(AddressType::FullData),
            0x04 => Ok(AddressType::FullType),
            _ => Err(format!("Invalid address type value: {}", value)),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum CodeHashIndex {
    // SECP256K1 + blake160
    Sighash = 0x00,
    // SECP256K1 + multisig
    Multisig = 0x01,
    // SECP256k1 + AnyoneCanPay
    AnyoneCanPay = 0x02,
}

impl CodeHashIndex {
    pub fn from_u8(value: u8) -> Result<CodeHashIndex, String> {
        match value {
            0x00 => Ok(CodeHashIndex::Sighash),
            0x01 => Ok(CodeHashIndex::Multisig),
            0x02 => Ok(CodeHashIndex::AnyoneCanPay),
            _ => Err(format!("Invalid code hash index value: {}", value)),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum AddressPayload {
    Short {
        net_ty: NetworkType,
        index: CodeHashIndex,
        hash: H160,
    },
    Full {
        hash_type: ScriptHashType,
        code_hash: packed::Byte32,
        args: Bytes,
    },
}

impl AddressPayload {
    pub fn new_short(net_ty: NetworkType, index: CodeHashIndex, hash: H160) -> AddressPayload {
        AddressPayload::Short {
            net_ty,
            index,
            hash,
        }
    }

    pub fn new_full(
        hash_type: ScriptHashType,
        code_hash: packed::Byte32,
        args: Bytes,
    ) -> AddressPayload {
        AddressPayload::Full {
            hash_type,
            code_hash,
            args,
        }
    }
    pub fn new_full_data(code_hash: packed::Byte32, args: Bytes) -> AddressPayload {
        Self::new_full(ScriptHashType::Data, code_hash, args)
    }
    pub fn new_full_type(code_hash: packed::Byte32, args: Bytes) -> AddressPayload {
        Self::new_full(ScriptHashType::Type, code_hash, args)
    }

    pub fn ty(&self, is_new: bool) -> AddressType {
        match self {
            AddressPayload::Short { .. } => AddressType::Short,
            AddressPayload::Full { hash_type, .. } => match (hash_type, is_new) {
                (ScriptHashType::Data, true) => AddressType::Full,
                (ScriptHashType::Data, false) => AddressType::FullData,
                (ScriptHashType::Data1, _) => AddressType::Full,
                (ScriptHashType::Type, true) => AddressType::Full,
                (ScriptHashType::Type, false) => AddressType::FullType,
            },
        }
    }

    pub fn hash_type(&self) -> ScriptHashType {
        match self {
            AddressPayload::Short { .. } => ScriptHashType::Type,
            AddressPayload::Full { hash_type, .. } => *hash_type,
        }
    }

    pub fn code_hash(&self) -> packed::Byte32 {
        match self {
            AddressPayload::Short { net_ty, index, .. } => match index {
                CodeHashIndex::Sighash => SIGHASH_TYPE_HASH.clone().pack(),
                CodeHashIndex::Multisig => MULTISIG_TYPE_HASH.clone().pack(),
                CodeHashIndex::AnyoneCanPay => {
                    if net_ty == &NetworkType::Mainnet {
                        ACP_MAINNET_TYPE_HASH.clone().pack()
                    } else {
                        ACP_TESTNET_TYPE_HASH.clone().pack()
                    }
                }
            },
            AddressPayload::Full { code_hash, .. } => code_hash.clone(),
        }
    }

    pub fn args(&self) -> Bytes {
        match self {
            AddressPayload::Short { hash, .. } => Bytes::from(hash.as_bytes().to_vec()),
            AddressPayload::Full { args, .. } => args.clone(),
        }
    }

    pub fn display_with_network(&self, network: NetworkType, is_new: bool) -> String {
        let hrp = network.to_prefix();
        let (data, varient) = match self {
            // payload = 0x01 | code_hash_index | args
            AddressPayload::Short { index, hash, .. } => {
                let mut data = vec![0u8; 22];
                data[0] = self.ty(is_new) as u8;
                data[1] = *index as u8;
                data[2..].copy_from_slice(hash.as_bytes());
                // short address always use bech32
                (data, bech32::Variant::Bech32)
            }

            AddressPayload::Full {
                code_hash,
                hash_type,
                args,
            } => {
                if is_new {
                    // payload = 0x00 | code_hash | hash_type | args
                    let mut data = vec![0u8; 34 + args.len()];
                    data[0] = self.ty(is_new) as u8;
                    data[1..33].copy_from_slice(code_hash.as_slice());
                    data[33] = (*hash_type) as u8;
                    data[34..].copy_from_slice(args.as_ref());
                    (data, bech32::Variant::Bech32m)
                } else {
                    // payload = 0x02/0x04 | code_hash | args
                    let mut data = vec![0u8; 33 + args.len()];
                    data[0] = self.ty(is_new) as u8;
                    data[1..33].copy_from_slice(code_hash.as_slice());
                    data[33..].copy_from_slice(args.as_ref());
                    (data, bech32::Variant::Bech32)
                }
            }
        };

        bech32::encode(hrp, data.to_base32(), varient)
            .unwrap_or_else(|_| panic!("Encode address failed: payload={:?}", self))
    }

    pub fn from_pubkey(net_ty: NetworkType, pubkey: &secp256k1::PublicKey) -> AddressPayload {
        // Serialize pubkey as compressed format
        let hash = H160::from_slice(&blake2b_256(&pubkey.serialize()[..])[0..20])
            .expect("Generate hash(H160) from pubkey failed");
        AddressPayload::from_pubkey_hash(net_ty, hash)
    }

    pub fn from_pubkey_hash(net_ty: NetworkType, hash: H160) -> AddressPayload {
        let index = CodeHashIndex::Sighash;
        AddressPayload::Short {
            net_ty,
            index,
            hash,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            AddressPayload::Short { index, hash, .. } => {
                let mut data = vec![0u8; 21];
                data[0] = (*index) as u8;
                data[1..21].copy_from_slice(hash.as_bytes());
                data
            }
            AddressPayload::Full {
                code_hash, args, ..
            } => {
                let mut data = vec![0u8; 32 + args.len()];
                data[0..32].copy_from_slice(code_hash.as_slice());
                data[32..].copy_from_slice(args.as_ref());
                data
            }
        }
    }

    #[allow(clippy::if_same_then_else)]
    pub fn from_script(lock: &packed::Script, net_ty: NetworkType) -> Self {
        let hash_type: ScriptHashType = lock.hash_type().try_into().expect("Invalid hash_type");
        let code_hash = lock.code_hash();
        let code_hash_h256: H256 = code_hash.unpack();
        let args = lock.args().raw_data();

        if hash_type == ScriptHashType::Type
            && code_hash_h256 == SIGHASH_TYPE_HASH
            && args.len() == 20
        {
            let index = CodeHashIndex::Sighash;
            let hash = H160::from_slice(args.as_ref()).unwrap();
            AddressPayload::Short {
                net_ty,
                index,
                hash,
            }
        } else if hash_type == ScriptHashType::Type
            && code_hash_h256 == MULTISIG_TYPE_HASH
            && args.len() == 20
        {
            let index = CodeHashIndex::Multisig;
            let hash = H160::from_slice(args.as_ref()).unwrap();
            AddressPayload::Short {
                net_ty,
                index,
                hash,
            }
        } else if hash_type == ScriptHashType::Type
            && net_ty == NetworkType::Mainnet
            && code_hash_h256 == ACP_MAINNET_TYPE_HASH
        {
            let index = CodeHashIndex::AnyoneCanPay;
            let hash = H160::from_slice(&args.as_ref()[0..20]).unwrap();
            AddressPayload::Short {
                net_ty,
                index,
                hash,
            }
        } else if hash_type == ScriptHashType::Type
            && net_ty == NetworkType::Testnet
            && code_hash_h256 == ACP_TESTNET_TYPE_HASH
        {
            let index = CodeHashIndex::AnyoneCanPay;
            let hash = H160::from_slice(&args.as_ref()[0..20]).unwrap();
            AddressPayload::Short {
                net_ty,
                index,
                hash,
            }
        } else {
            AddressPayload::Full {
                hash_type,
                code_hash,
                args,
            }
        }
    }
}

impl fmt::Debug for AddressPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hash_type = if self.hash_type() == ScriptHashType::Type {
            "type"
        } else {
            "data"
        };
        f.debug_struct("AddressPayload")
            .field("hash_type", &hash_type)
            .field("code_hash", &self.code_hash())
            .field("args", &self.args())
            .finish()
    }
}

impl From<&AddressPayload> for packed::Script {
    fn from(payload: &AddressPayload) -> packed::Script {
        packed::Script::new_builder()
            .hash_type(payload.hash_type().into())
            .code_hash(payload.code_hash())
            .args(payload.args().pack())
            .build()
    }
}

impl From<packed::Script> for AddressPayload {
    #[allow(clippy::fallible_impl_from)]
    fn from(lock: packed::Script) -> AddressPayload {
        let hash_type: ScriptHashType = lock.hash_type().try_into().expect("Invalid hash_type");
        let code_hash = lock.code_hash();
        let code_hash_h256: H256 = code_hash.unpack();
        let args = lock.args().raw_data();
        let net_ty = NetworkType::Mainnet;

        if hash_type == ScriptHashType::Type
            && code_hash_h256 == SIGHASH_TYPE_HASH
            && args.len() == 20
        {
            let index = CodeHashIndex::Sighash;
            let hash = H160::from_slice(args.as_ref()).unwrap();
            AddressPayload::Short {
                net_ty,
                index,
                hash,
            }
        } else if hash_type == ScriptHashType::Type
            && code_hash_h256 == MULTISIG_TYPE_HASH
            && args.len() == 20
        {
            let index = CodeHashIndex::Multisig;
            let hash = H160::from_slice(args.as_ref()).unwrap();
            AddressPayload::Short {
                net_ty,
                index,
                hash,
            }
        } else {
            AddressPayload::Full {
                hash_type,
                code_hash,
                args,
            }
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Address {
    network: NetworkType,
    payload: AddressPayload,
    pub(crate) is_new: bool,
}

impl Address {
    pub fn new(network: NetworkType, payload: AddressPayload, is_new: bool) -> Address {
        Address {
            network,
            payload,
            is_new,
        }
    }

    pub fn network(&self) -> NetworkType {
        self.network
    }

    pub fn payload(&self) -> &AddressPayload {
        &self.payload
    }

    pub fn is_secp256k1(&self) -> bool {
        match &self.payload {
            AddressPayload::Short { index, .. } => index == &CodeHashIndex::Sighash,
            AddressPayload::Full {
                hash_type,
                code_hash,
                ..
            } => hash_type == &ScriptHashType::Type && code_hash == &SIGHASH_TYPE_HASH.pack(),
        }
    }

    pub fn is_acp(&self) -> bool {
        match &self.payload {
            AddressPayload::Short { index, .. } => index == &CodeHashIndex::AnyoneCanPay,
            AddressPayload::Full {
                hash_type,
                code_hash,
                ..
            } => match self.network {
                NetworkType::Mainnet => {
                    hash_type == &ScriptHashType::Type && code_hash == &ACP_MAINNET_TYPE_HASH.pack()
                }
                NetworkType::Testnet => {
                    hash_type == &ScriptHashType::Type && code_hash == &ACP_TESTNET_TYPE_HASH.pack()
                }
                _ => false,
            },
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            self.payload.display_with_network(self.network, self.is_new)
        )
    }
}

impl FromStr for Address {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (hrp, value, variant) = bech32::decode(input).map_err(|err| err.to_string())?;
        let net_ty =
            NetworkType::from_prefix(&hrp).ok_or_else(|| format!("Invalid hrp: {}", hrp))?;
        let network = net_ty;
        let data = convert_bits(&value, 5, 8, false).unwrap();
        let ty = AddressType::from_u8(data[0])?;
        match ty {
            // payload = 0x01 | code_hash_index | args
            AddressType::Short => {
                if variant != Variant::Bech32 {
                    return Err("short address must use bech32 encoding".to_string());
                }
                if data.len() != 22 {
                    return Err(format!("Invalid input data length {}", data.len()));
                }
                let index = CodeHashIndex::from_u8(data[1])?;
                let hash = H160::from_slice(&data[2..22]).unwrap();
                let payload = AddressPayload::Short {
                    index,
                    hash,
                    net_ty,
                };
                Ok(Address {
                    network,
                    payload,
                    is_new: false,
                })
            }

            // payload = 0x02/0x04 | code_hash | args
            AddressType::FullData | AddressType::FullType => {
                if variant != Variant::Bech32 {
                    return Err(
                        "non-ckb2021 format full address must use bech32 encoding".to_string()
                    );
                }
                if data.len() < 33 {
                    return Err(format!("Insufficient data length: {}", data.len()));
                }
                let hash_type = if ty == AddressType::FullData {
                    ScriptHashType::Data
                } else {
                    ScriptHashType::Type
                };
                let code_hash = packed::Byte32::from_slice(&data[1..33]).unwrap();
                let args = Bytes::from(data[33..].to_vec());
                let payload = AddressPayload::Full {
                    hash_type,
                    code_hash,
                    args,
                };
                Ok(Address {
                    network,
                    payload,
                    is_new: false,
                })
            }

            // payload = 0x00 | code_hash | hash_type | args
            AddressType::Full => {
                if variant != Variant::Bech32m {
                    return Err("ckb2021 format full address must use bech32m encoding".to_string());
                }
                if data.len() < 34 {
                    return Err(format!("Insufficient data length: {}", data.len()));
                }
                let code_hash = packed::Byte32::from_slice(&data[1..33]).unwrap();
                let hash_type =
                    ScriptHashType::try_from(data[33]).map_err(|err| err.to_string())?;
                let args = Bytes::from(data[34..].to_vec());
                let payload = AddressPayload::Full {
                    hash_type,
                    code_hash,
                    args,
                };
                Ok(Address {
                    network,
                    payload,
                    is_new: true,
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ckb_types::{h160, h256};

    #[test]
    fn test_short_address() {
        let payload = AddressPayload::from_pubkey_hash(
            NetworkType::Mainnet,
            h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64"),
        );
        let address = Address::new(NetworkType::Mainnet, payload, false);
        assert_eq!(
            address.to_string(),
            "ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v"
        );
        assert_eq!(
            address,
            Address::from_str("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v").unwrap()
        );

        let payload = AddressPayload::from_pubkey_hash(
            NetworkType::Mainnet,
            h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64"),
        );
        let address = Address::new(NetworkType::Mainnet, payload, true);
        assert_eq!(
            address.to_string(),
            "ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v"
        );

        let index = CodeHashIndex::Multisig;
        let payload = AddressPayload::new_short(
            NetworkType::Mainnet,
            index,
            h160!("0x4fb2be2e5d0c1a3b8694f832350a33c1685d477a"),
        );
        let address = Address::new(NetworkType::Mainnet, payload, false);
        assert_eq!(
            address.to_string(),
            "ckb1qyq5lv479ewscx3ms620sv34pgeuz6zagaaqklhtgg"
        );
        assert_eq!(
            address,
            Address::from_str("ckb1qyq5lv479ewscx3ms620sv34pgeuz6zagaaqklhtgg").unwrap()
        );
        let acp_address_str = "ckb1qypzygjgr5425uvg2jcq3c7cxvpuv0rp4nssh7wm4f";
        let payload = AddressPayload::new_short(
            NetworkType::Mainnet,
            CodeHashIndex::AnyoneCanPay,
            h160!("0x2222481d2aaa718854b008e3d83303c63c61ace1"),
        );
        let acp_address = Address::new(NetworkType::Mainnet, payload, false);
        assert_eq!(acp_address.to_string(), acp_address_str);
        assert_eq!(acp_address, Address::from_str(acp_address_str).unwrap());
    }

    #[test]
    fn test_old_full_address() {
        let hash_type = ScriptHashType::Type;
        let code_hash = packed::Byte32::from_slice(
            h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").as_bytes(),
        )
        .unwrap();
        let args = Bytes::from(h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64").as_bytes());
        let payload = AddressPayload::new_full(hash_type, code_hash, args);
        let address = Address::new(NetworkType::Mainnet, payload, false);

        assert_eq!(address.to_string(), "ckb1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3vumhs9nvu786dj9p0q5elx66t24n3kxgj53qks");
        assert_eq!(address, Address::from_str("ckb1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3vumhs9nvu786dj9p0q5elx66t24n3kxgj53qks").unwrap());
    }

    #[test]
    fn test_new_full_address() {
        let code_hash = packed::Byte32::from_slice(
            h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").as_bytes(),
        )
        .unwrap();
        let args = Bytes::from(h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64").as_bytes());

        let payload =
            AddressPayload::new_full(ScriptHashType::Type, code_hash.clone(), args.clone());
        let address = Address::new(NetworkType::Mainnet, payload, true);
        assert_eq!(address.to_string(), "ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdnnw7qkdnnclfkg59uzn8umtfd2kwxceqxwquc4");
        assert_eq!(address, Address::from_str("ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdnnw7qkdnnclfkg59uzn8umtfd2kwxceqxwquc4").unwrap());

        let payload =
            AddressPayload::new_full(ScriptHashType::Data, code_hash.clone(), args.clone());
        let address = Address::new(NetworkType::Mainnet, payload, true);
        assert_eq!(address.to_string(), "ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq9nnw7qkdnnclfkg59uzn8umtfd2kwxceqvguktl");
        assert_eq!(address, Address::from_str("ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq9nnw7qkdnnclfkg59uzn8umtfd2kwxceqvguktl").unwrap());

        let payload = AddressPayload::new_full(ScriptHashType::Data1, code_hash, args);
        let address = Address::new(NetworkType::Mainnet, payload, true);
        assert_eq!(address.to_string(), "ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq4nnw7qkdnnclfkg59uzn8umtfd2kwxceqcydzyt");
        assert_eq!(address, Address::from_str("ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq4nnw7qkdnnclfkg59uzn8umtfd2kwxceqcydzyt").unwrap());
    }
}
