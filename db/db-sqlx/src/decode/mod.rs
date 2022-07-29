use ckb_types::H256;
use sqlx_core::database::{Database, HasValueRef};
use sqlx_core::decode::Decode;
use sqlx_core::type_info::TypeInfo;
use sqlx_core::types::Type;

use std::error::Error;
use std::fmt;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct H256_(pub H256);

// impl sqlx::Type<Postgres> for H256_ {
//     fn type_info() -> sqlx::postgres::PgTypeInfo {
//         sqlx::postgres::PgTypeInfo::with_name("month_id")
//     }
// }

impl fmt::Display for H256_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl<DB: Database> Type<DB> for H256_ {
    fn type_info() -> DB::TypeInfo {
        todo!()
    }
}

impl std::str::FromStr for H256_ {
    type Err = sqlx_core::error::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = H256::from_str(s).map_err(|e| ::sqlx::Error::Decode(e.to_string().into()))?;
        Ok(H256_(hash))
    }
}

// DB is the database driver
// `'r` is the lifetime of the `Row` being decoded
impl<'r, DB: Database> Decode<'r, DB> for H256_
where
    // we want to delegate some of the work to string decoding so let's make sure strings
    // are supported by the database
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<H256_, Box<dyn Error + 'static + Send + Sync>> {
        // the interface of ValueRef is largely unstable at the moment
        // so this is not directly implementable

        // however, you can delegate to a type that matches the format of the type you want
        // to decode (such as a UTF-8 string)

        let value = <&str as Decode<DB>>::decode(value)?;

        // now you can parse this into your type (assuming there is a `FromStr`)

        Ok(value.parse()?)
    }
}

impl TypeInfo for H256_ {
    fn is_null(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "H256_"
    }
}
