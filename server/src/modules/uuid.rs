// STDB Discord: https://discord.gg/7k3v8a4
// https://discord.com/channels/1037340874172014652/1361787955076665607
//
// === SpacetimeDB UUID implementation ===
use std::{fmt::Display, ops::Deref};

use spacetimedb::{
    sats::{impl_deserialize, impl_serialize, impl_st},
    ReducerContext
};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone,PartialEq)]
pub struct StUuid(Uuid);

impl_st!([] StUuid, spacetimedb::sats::AlgebraicType::String);
impl_serialize!([] StUuid, (self, ser) => {
    ser.serialize_str(self.hyphenated().encode_upper(&mut Uuid::encode_buffer()))
});
impl_deserialize!([] StUuid, de => {
    let s = String::deserialize(de).map(|s| s.into_boxed_str())?;
    Ok(Uuid::parse_str(&s).map(|u| u.into()).expect("Failed to Deserialize to UUID"))
});
impl StUuid {
    pub fn new(ctx: &ReducerContext) -> Self {
        StUuid(Uuid::from_bytes(ctx.random()))
    }
}

impl Display for StUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl Deref for StUuid {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for StUuid {
    fn from(val: Uuid) -> Self {
        Self(val)
    }
}

