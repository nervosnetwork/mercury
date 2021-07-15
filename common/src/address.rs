use crate::{NetworkType, ACP_TYPE_HASH, MULTISIG_TYPE_HASH, SIGHASH_TYPE_HASH};

use bech32::{convert_bits, Bech32, ToBase32};
use ckb_hash::blake2b_256;
use ckb_types::{bytes::Bytes, core::ScriptHashType, packed, prelude::*, H160, H256};
use serde_derive::{Deserialize, Serialize};

use std::convert::TryInto;
use std::fmt;
use std::str::FromStr;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum AddressType {
    Short = 0x01,
    FullData = 0x02,
    FullType = 0x04,
}

impl AddressType {
    pub fn from_u8(value: u8) -> Result<AddressType, String> {
        match value {
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
    pub fn new_short(index: CodeHashIndex, hash: H160) -> AddressPayload {
        AddressPayload::Short { index, hash }
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

    pub fn ty(&self) -> AddressType {
        match self {
            AddressPayload::Short { .. } => AddressType::Short,
            AddressPayload::Full { hash_type, .. } => match hash_type {
                ScriptHashType::Data => AddressType::FullData,
                ScriptHashType::Type => AddressType::FullType,
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
            AddressPayload::Short { index, .. } => match index {
                CodeHashIndex::Sighash => SIGHASH_TYPE_HASH.clone().pack(),
                CodeHashIndex::Multisig => MULTISIG_TYPE_HASH.clone().pack(),
                CodeHashIndex::AnyoneCanPay => ACP_TYPE_HASH.clone().pack(),
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

    pub fn from_pubkey(pubkey: &secp256k1::PublicKey) -> AddressPayload {
        // Serialize pubkey as compressed format
        let hash = H160::from_slice(&blake2b_256(&pubkey.serialize()[..])[0..20])
            .expect("Generate hash(H160) from pubkey failed");
        AddressPayload::from_pubkey_hash(hash)
    }

    pub fn from_pubkey_hash(hash: H160) -> AddressPayload {
        let index = CodeHashIndex::Sighash;
        AddressPayload::Short { index, hash }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            AddressPayload::Short { index, hash } => {
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
        if hash_type == ScriptHashType::Type
            && code_hash_h256 == SIGHASH_TYPE_HASH
            && args.len() == 20
        {
            let index = CodeHashIndex::Sighash;
            let hash = H160::from_slice(args.as_ref()).unwrap();
            AddressPayload::Short { index, hash }
        } else if hash_type == ScriptHashType::Type
            && code_hash_h256 == MULTISIG_TYPE_HASH
            && args.len() == 20
        {
            let index = CodeHashIndex::Multisig;
            let hash = H160::from_slice(args.as_ref()).unwrap();
            AddressPayload::Short { index, hash }
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
}

impl Address {
    pub fn new(network: NetworkType, payload: AddressPayload) -> Address {
        Address { network, payload }
    }

    pub fn network(&self) -> NetworkType {
        self.network
    }

    pub fn payload(&self) -> &AddressPayload {
        &self.payload
    }

    pub fn display_with_network(&self, network: NetworkType) -> String {
        let hrp = network.to_prefix();
        let data = match self.payload.ty() {
            AddressType::Short => {
                let mut data = vec![0; 22];
                data[0] = self.payload.ty() as u8;
                data[1..].copy_from_slice(self.payload.to_bytes().as_slice());
                data
            }
            AddressType::FullData | AddressType::FullType => {
                let payload_data = self.payload.to_bytes();
                let mut data = vec![0; payload_data.len() + 1];
                data[0] = self.payload.ty() as u8;
                data[1..].copy_from_slice(payload_data.as_slice());
                data
            }
        };
        let value = Bech32::new(hrp.to_string(), data.to_base32())
            .unwrap_or_else(|_| panic!("Encode address failed: payload={:?}", self.payload));
        format!("{}", value)
    }

    pub fn is_secp256k1(&self) -> bool {
        match self.payload {
            AddressPayload::Short { index, .. } => index == CodeHashIndex::Sighash,
            AddressPayload::Full {
                hash_type,
                code_hash,
                ..
            } => hash_type == ScriptHashType::Type && code_hash == SIGHASH_TYPE_HASH.pack(),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.display_with_network(self.network))
    }
}

impl FromStr for Address {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let value = Bech32::from_str(input).map_err(|err| err.to_string())?;
        let network = NetworkType::from_prefix(value.hrp())
            .ok_or_else(|| format!("Invalid hrp: {}", value.hrp()))?;
        let data = convert_bits(value.data(), 5, 8, false).unwrap();
        let ty = AddressType::from_u8(data[0])?;
        match ty {
            AddressType::Short => {
                if data.len() != 22 {
                    return Err(format!("Invalid input data length {}", data.len()));
                }
                let index = CodeHashIndex::from_u8(data[1])?;
                let hash = H160::from_slice(&data[2..22]).unwrap();
                let payload = AddressPayload::Short { index, hash };
                Ok(Address { network, payload })
            }
            AddressType::FullData | AddressType::FullType => {
                if data.len() < 32 {
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
                Ok(Address { network, payload })
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
        let payload =
            AddressPayload::from_pubkey_hash(h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64"));
        let address = Address::new(NetworkType::Mainnet, payload);
        assert_eq!(
            address.to_string(),
            "ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v"
        );
        assert_eq!(
            address,
            Address::from_str("ckb1qyqt8xaupvm8837nv3gtc9x0ekkj64vud3jqfwyw5v").unwrap()
        );

        let index = CodeHashIndex::Multisig;
        let payload =
            AddressPayload::new_short(index, h160!("0x4fb2be2e5d0c1a3b8694f832350a33c1685d477a"));
        let address = Address::new(NetworkType::Mainnet, payload);
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
            CodeHashIndex::AnyoneCanPay,
            h160!("0x2222481d2aaa718854b008e3d83303c63c61ace1"),
        );
        let acp_address = Address::new(NetworkType::Mainnet, payload);
        assert_eq!(acp_address.to_string(), acp_address_str);
        assert_eq!(acp_address, Address::from_str(acp_address_str).unwrap());
    }

    #[test]
    fn test_full_address() {
        let hash_type = ScriptHashType::Type;
        let code_hash = packed::Byte32::from_slice(
            h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").as_bytes(),
        )
        .unwrap();
        let args = Bytes::from(h160!("0xb39bbc0b3673c7d36450bc14cfcdad2d559c6c64").as_bytes());
        let payload = AddressPayload::new_full(hash_type, code_hash, args);
        let address = Address::new(NetworkType::Mainnet, payload);
        assert_eq!(address.to_string(), "ckb1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3vumhs9nvu786dj9p0q5elx66t24n3kxgj53qks");
        assert_eq!(address, Address::from_str("ckb1qjda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xw3vumhs9nvu786dj9p0q5elx66t24n3kxgj53qks").unwrap());
    }
}
