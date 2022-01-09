use crate::{
    NetworkType, ACP_MAINNET_TYPE_HASH, ACP_TESTNET_TYPE_HASH, MULTISIG_TYPE_HASH,
    PW_LOCK_MAINNET_TYPE_HASH, PW_LOCK_TESTNET_TYPE_HASH, SIGHASH_TYPE_HASH,
};

use bech32::{convert_bits, ToBase32, Variant};
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, packed, prelude::*, H160};
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
    #[deprecated]
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
        AddressPayload::new_full(ScriptHashType::Data, code_hash, args)
    }

    pub fn new_full_type(code_hash: packed::Byte32, args: Bytes) -> AddressPayload {
        AddressPayload::new_full(ScriptHashType::Type, code_hash, args)
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

    pub fn from_pubkey(pubkey: &secp256k1::PublicKey) -> AddressPayload {
        // Serialize pubkey as compressed format
        let hash = H160::from_slice(&blake2b_256(&pubkey.serialize()[..])[0..20])
            .expect("Generate hash(H160) from pubkey failed");
        AddressPayload::from_pubkey_hash(hash)
    }

    pub fn from_pubkey_hash(hash: H160) -> AddressPayload {
        AddressPayload::Full {
            hash_type: ScriptHashType::Type,
            code_hash: SIGHASH_TYPE_HASH.pack(),
            args: Bytes::from(hash.as_bytes().to_vec()),
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

    pub fn from_script(lock: &packed::Script) -> Self {
        AddressPayload::Full {
            hash_type: lock.hash_type().try_into().expect("Invalid hash_type"),
            code_hash: lock.code_hash(),
            args: lock.args().raw_data(),
        }
    }
}

impl fmt::Debug for AddressPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hash_type = match self.hash_type() {
            ScriptHashType::Type => "type",
            ScriptHashType::Data => "data",
            ScriptHashType::Data1 => "data1",
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
    fn from(lock: packed::Script) -> AddressPayload {
        let hash_type: ScriptHashType = lock.hash_type().try_into().expect("Invalid hash_type");
        let code_hash = lock.code_hash();
        let args = lock.args().raw_data();

        AddressPayload::Full {
            hash_type,
            code_hash,
            args,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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

    pub fn is_pw_lock(&self) -> bool {
        match &self.payload {
            AddressPayload::Short { .. } => false,
            AddressPayload::Full {
                hash_type,
                code_hash,
                ..
            } => match self.network {
                NetworkType::Mainnet => {
                    hash_type == &ScriptHashType::Type
                        && code_hash == &PW_LOCK_MAINNET_TYPE_HASH.pack()
                }
                NetworkType::Testnet => {
                    hash_type == &ScriptHashType::Type
                        && code_hash == &PW_LOCK_TESTNET_TYPE_HASH.pack()
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
    use crypto::digest::Digest;

    #[test]
    #[allow(deprecated)]
    fn test_short_address() {
        let payload =
            AddressPayload::from_pubkey_hash(h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64"));

        let short_payload = AddressPayload::new_short(
            NetworkType::Mainnet,
            CodeHashIndex::Sighash,
            h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64"),
        );
        let address = Address::new(NetworkType::Mainnet, payload, false);
        assert_eq!(
            address.to_string(),
            "ckb1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3vumhs9nvu786dj9p0q5elx66t24n3kxgj53qks"
        );
        assert_eq!(
            Address::new(NetworkType::Mainnet, short_payload, false),
            Address::from_str("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v").unwrap()
        );

        let payload =
            AddressPayload::from_pubkey_hash(h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64"));
        let address = Address::new(NetworkType::Mainnet, payload, true);
        assert_eq!(
            address.to_string(),
            "ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdnnw7qkdnnclfkg59uzn8umtfd2kwxceqxwquc4"
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

    #[test]
    fn test_args_generation_for_secp_lock() {
        // compressed pubkey(33 bytes) from privkey
        let pubkey = secp256k1::PublicKey::from_str(
            "03925521a821472173f29716378f829b5d35a2e614329cc52a9c0ad5520e8f15bd",
        )
        .unwrap();
        println!("pubkey: {:?}", pubkey);
        let pubkey = &pubkey.serialize()[..];
        assert_eq!(33, pubkey.len());
        println!("pubkey: {:?}", pubkey);

        // pubkey hash
        let pubkey_hash = blake2b_256(pubkey);
        assert_eq!(32, pubkey_hash.len());
        println!("pubkey_hash: {:?}", pubkey_hash);

        // generate lock args by pubkey hash
        let pubkey_hash = &pubkey_hash[0..20];
        println!("pubkey_hash: {:?}", pubkey_hash);
        let pubkey_hash =
            H160::from_slice(pubkey_hash).expect("Generate hash(H160) from pubkey failed");
        println!("pubkey_hash: {:?}", pubkey_hash);
        let args = Bytes::from(pubkey_hash.as_bytes().to_vec());
        assert_eq!(
            "bb6f5e0696fcb7e832ab920be62e6b03af45be35".to_string(),
            hex::encode(args.clone())
        );

        // secp address
        let secp_code_hash = packed::Byte32::from_slice(
            h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").as_bytes(),
        )
        .unwrap();
        let payload =
            AddressPayload::new_full(ScriptHashType::Type, secp_code_hash.clone(), args.clone());
        let address = Address::new(NetworkType::Testnet, payload.clone(), true);
        assert_eq!("ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdmda0qd9hukl5r92ujp0nzu6cr4azmudgur5kut".to_string(), address.to_string());
        let address = Address::new(NetworkType::Testnet, payload, false);
        assert_eq!("ckt1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3wm0tcrfdl9haqe2hystuchxkqa0gklr24t5h4x".to_string(), address.to_string());

        // acp address
        let acp_code_hash = packed::Byte32::from_slice(
            h256!("0x3419a1c09eb2567f6552ee7a8ecffd64155cffe0f1796e6e61ec088d740c1356").as_bytes(),
        )
        .unwrap();
        let payload =
            AddressPayload::new_full(ScriptHashType::Type, acp_code_hash.clone(), args.clone());
        let address = Address::new(NetworkType::Testnet, payload.clone(), true);
        assert_eq!("ckt1qq6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4vqdmda0qd9hukl5r92ujp0nzu6cr4azmudgnsjjfa".to_string(), address.to_string());
        let address = Address::new(NetworkType::Testnet, payload, false);
        assert_eq!("ckt1qs6pngwqn6e9vlm92th84rk0l4jp2h8lurchjmnwv8kq3rt5psf4dwm0tcrfdl9haqe2hystuchxkqa0gklr2y5cqpz".to_string(), address.to_string());
    }

    #[test]
    fn test_eth_address_generation() {
        // data from book Mastering Ethereum
        let pubkey_addr_pair = ("046e145ccef1033dea239875dd00dfb4fee6e3348b84985c92f103444683bae07b83b5c38e5e2b0c8529d7fa3f64d46daa1ece2d9ac14cab9477d042c84c32ccd0", "0x001d3f1ef827552ae1114027bd3ecf1f086ba0f9");

        let pubkey_with_prefix = pubkey_addr_pair.0;
        let eth_address = pubkey_to_eth_address(pubkey_with_prefix);
        let mut address = "0x".to_owned();
        address.push_str(&eth_address);
        assert_eq!(pubkey_addr_pair.1.to_owned(), address);
    }

    #[test]
    fn test_eth_address_generation_from_privkey() {
        // eth address generated by third tool
        let pk_addr_pair = (
            "53823e223958d89b97ab4de71896e67d739a28bad596574f958498a00bd11d95",
            "0x978f92103BE30Da611eC95a29a1f33dEF059BAC3",
        );

        // generate pubkey by privkey
        let secret_key = secp256k1::SecretKey::from_str(pk_addr_pair.0).unwrap();
        let secp256k1: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
        let pubkey = secp256k1::PublicKey::from_secret_key(&secp256k1, &secret_key);
        println!("pubkey: {:?}", pubkey);
        let pubkey = hex::encode(pubkey.serialize_uncompressed());
        assert_eq!("04bf2cdbbce3731e19e284424b05bea5cd426aa80771da68f054efdc2e93cc2bada56d0d54e5ee5aa6fd259bb84e5e79100a4f53e14b8952fb4c9931d9074303d0".to_string(), pubkey);

        // generate eth address by uncompressed pubkey
        let eth_address = pubkey_to_eth_address(&pubkey);
        let mut address = "0x".to_owned();
        address.push_str(&eth_address);
        assert_eq!(pk_addr_pair.1.to_lowercase(), address);
    }

    #[test]
    fn test_args_generation_for_pw_lock() {
        // compressed pubkey(33 bytes)
        let pubkey_ori = secp256k1::PublicKey::from_str(
            "03925521a821472173f29716378f829b5d35a2e614329cc52a9c0ad5520e8f15bd",
        )
        .unwrap();
        println!("pubkey_ori: {:?}", pubkey_ori);
        let pubkey_compressed = &pubkey_ori.serialize()[..];
        assert_eq!(33, pubkey_compressed.len());
        println!("pubkey_compressed: {:?}", hex::encode(pubkey_compressed));
        let pubkey_uncompressed = hex::encode(pubkey_ori.serialize_uncompressed());
        println!("pubkey_uncompressed: {:?}", pubkey_uncompressed);

        // secp lock args
        let secp_lock_args = pubkey_to_secp_lock_arg(&pubkey_uncompressed);
        assert_eq!(
            "bb6f5e0696fcb7e832ab920be62e6b03af45be35".to_string(),
            secp_lock_args
        );

        // pw lock args
        let pw_lock_args = pubkey_to_eth_address(&pubkey_uncompressed);
        println!("pw_lock_args: {:?}", pw_lock_args);
        assert_eq!(
            "adabffb9c27cb4af100ce7bca6903315220e87a2".to_string(),
            pw_lock_args
        );

        assert_ne!(secp_lock_args, pw_lock_args);

        // pw lock address
        let pw_lock_code_hash = packed::Byte32::from_slice(
            h256!("0x58c5f491aba6d61678b7cf7edf4910b1f5e00ec0cde2f42e0abb4fd9aff25a63").as_bytes(),
        )
        .unwrap();
        let args = Bytes::from(h160!("0xadabffb9c27cb4af100ce7bca6903315220e87a2").as_bytes());
        let payload = AddressPayload::new_full(
            ScriptHashType::Type,
            pw_lock_code_hash.clone(),
            args.clone(),
        );
        let pw_lock_address = Address::new(NetworkType::Testnet, payload.clone(), true);
        println!("pw_lock_address: {:?}", pw_lock_address.to_string());
        assert_eq!("ckt1qpvvtay34wndv9nckl8hah6fzzcltcqwcrx79apwp2a5lkd07fdxxqdd40lmnsnukjh3qr88hjnfqvc4yg8g0gskp8ffv".to_string(), pw_lock_address.to_string());
        let pw_lock_address_old = Address::new(NetworkType::Testnet, payload, false);
        assert_eq!("ckt1q3vvtay34wndv9nckl8hah6fzzcltcqwcrx79apwp2a5lkd07fdx8tdtl7uuyl954ugqeeau56grx9fzp6r6yjcyvrv".to_string(), pw_lock_address_old.to_string());
    }

    fn pubkey_to_secp_lock_arg(pubkey_uncompressed: &str) -> String {
        let pubkey = secp256k1::PublicKey::from_str(pubkey_uncompressed).unwrap();
        let pubkey_compressed = &pubkey.serialize()[..];
        assert_eq!(33, pubkey_compressed.len());
        let pubkey_hash = blake2b_256(pubkey_compressed);
        assert_eq!(32, pubkey_hash.len());
        println!("pubkey_hash: {:?}", pubkey_hash);
        let pubkey_hash = &pubkey_hash[0..20];
        println!("pubkey_hash: {:?}", pubkey_hash);
        let pubkey_hash =
            H160::from_slice(pubkey_hash).expect("Generate hash(H160) from pubkey failed");
        println!("pubkey_hash: {:?}", pubkey_hash);
        let args = Bytes::from(pubkey_hash.as_bytes().to_vec());
        hex::encode(args.clone())
    }

    fn pubkey_to_eth_address(pubkey_uncompressed: &str) -> String {
        assert_eq!(130, pubkey_uncompressed.chars().count());
        let pubkey_without_prefix = pubkey_uncompressed.split_once("04").unwrap().1;
        let pubkey_without_prefix = hex::decode(pubkey_without_prefix).unwrap();
        let mut hasher = crypto::sha3::Sha3::keccak256();
        hasher.input(&pubkey_without_prefix);
        let hash = hasher.result_str();
        hash.split_at(24).1.to_string()
    }
}
