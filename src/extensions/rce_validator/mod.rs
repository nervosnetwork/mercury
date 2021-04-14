mod generated;

use crate::{
    error::Error,
    extensions::{rce_validator::generated::xudt_rce::SmtUpdate, Extension},
    types::DeployedScriptConfig,
};
use anyhow::Result;
use ckb_indexer::store::{Batch, Store};
use ckb_types::{
    core::{BlockNumber, BlockView},
    packed::{Byte32, Bytes, WitnessArgs},
};
use molecule::prelude::Entity;

pub struct RceValidatorExtension<S> {
    store: S,
    config: DeployedScriptConfig,
}

impl<S> RceValidatorExtension<S> {
    pub fn new(store: S, config: DeployedScriptConfig) -> Self {
        Self { store, config }
    }

    // pub fn store(&self) -> &S {
    //     &self.store
    // }
}

pub enum Key<'a> {
    Address(&'a Bytes, &'a Byte32),
}

#[repr(u8)]
pub enum KeyPrefix {
    Address = 255,
}

// impl<'a> Key<'a> {
//     pub fn into_vec(self) -> Vec<u8> {
//         self.into()
//     }
// }

impl<'a> Into<Vec<u8>> for Key<'a> {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();

        match self {
            Key::Address(script_args, key) => {
                encoded.push(KeyPrefix::Address as u8);
                encoded.extend_from_slice(script_args.as_slice());
                encoded.extend_from_slice(key.as_slice());
            }
        }

        encoded
    }
}

pub enum Value {
    Presence(bool),
}

impl Into<Vec<u8>> for Value {
    fn into(self) -> Vec<u8> {
        let mut encoded = Vec::new();
        match self {
            Value::Presence(presense) => {
                if presense {
                    encoded.push(1);
                } else {
                    encoded.push(0);
                }
            }
        }
        encoded
    }
}

// impl Value {
//     pub fn parse_presence(slice: &[u8]) -> bool {
//         if slice[0] == 1 {
//             true
//         } else {
//             false
//         }
//     }
// }

impl<S> Extension for RceValidatorExtension<S>
where
    S: Store,
{
    fn append(&self, block: &BlockView) -> Result<()> {
        let mut batch = self.store.batch().map_err(|e| Error::from(e))?;
        for tx in block.data().transactions() {
            for (i, output) in tx.raw().outputs().into_iter().enumerate() {
                if let Some(type_script) = output.type_().to_opt() {
                    if type_script.code_hash() == self.config.script.code_hash()
                        && type_script.hash_type() == self.config.script.hash_type()
                    {
                        let witness = tx.witnesses().get(i).expect("invalid witness");
                        let witness_args = WitnessArgs::from_slice(&witness.raw_data())
                            .expect("invalid witness format");
                        let output_type = witness_args
                            .output_type()
                            .to_opt()
                            .expect("invalid witness output type");
                        let smt_update = SmtUpdate::from_slice(&output_type.raw_data())
                            .expect("invalid smt update");

                        for item in smt_update.update() {
                            let presence = u8::from(item.values()) & 0xF == 0x1;
                            let script_args = type_script.args();
                            let key = item.key();
                            batch
                                .put_kv(Key::Address(&script_args, &key), Value::Presence(presence))
                                .map_err(|e| Error::from(e))?;
                        }

                        // TODO: rollback data
                    }
                }
            }
        }
        batch.commit().map_err(|e| Error::from(e))?;
        Ok(())
    }

    fn rollback(&self, _tip_number: BlockNumber, _tip_hash: &Byte32) -> Result<()> {
        unimplemented!()
    }

    fn prune(&self, _keep_num: u64) -> Result<()> {
        unimplemented!()
    }
}
