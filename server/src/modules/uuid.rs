// === SpacetimeDB UUID implementation ===
use std::{fmt::Display, ops::Deref};

use spacetimedb::{
    sats::{impl_deserialize, impl_serialize, impl_st},
    ReducerContext
};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
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

// ===/ SpacetimeDB UUID implementation /===

/* 
#[spacetimedb::table(name = person)]
pub struct Person {
    #[primary_key]
    uuid: StUuid,
    name: String,
}

#[spacetimedb::reducer]
pub fn add(ctx: &ReducerContext, name: String) {
    ctx.db.person().insert(Person {
        name,
        uuid: StUuid::new(ctx),
    });
}

#[spacetimedb::reducer]
pub fn say_hello(ctx: &ReducerContext) {
    for person in ctx.db.person().iter() {
        log::info!("Hello, {} - {}!", person.name, person.uuid);
    }
    log::info!("Hello, World!");
}

*/