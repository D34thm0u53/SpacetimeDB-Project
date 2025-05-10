use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};

// Store current location of users
#[table(name = chunk, public)]
pub struct Chunk {
    #[primary_key]
    identity: Identity,
    x: u32,
    y: u32,
    z: u32,
}

// Structure for the entity Transform table
#[spacetimedb::table(name = stdb_transform, public)]
#[derive(Clone)]
pub struct StdbTransform {
    position: StdbPosition,
    rotation: StdbRotation,
}

// Structure for the entity position table
#[spacetimedb::table(name = stdb_position, public)]
#[derive(Clone)]
pub struct StdbPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
// Structure for the entity rotation table
#[spacetimedb::table(name = stdb_rotation, public)]
#[derive(Clone)]
pub struct StdbRotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Structure for the player entity table
#[table(name = player_entity, public)]
pub struct PlayerEntity {
    #[primary_key]
    identity: Identity,
    transform: StdbTransform,
}

// Structure for the non-player entity table
#[spacetimedb::table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    identity: Identity,
    transform: StdbTransform,
}


#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, transform: StdbTransform) {
    // The user has provided us with an update of their current position
    
    if let Some(player_entity) = ctx.db.player_entity().identity().find(ctx.sender) {
        // Update the user's internal position
        ctx.db.player_entity().identity().update(PlayerEntity { 
            transform,
            ..player_entity
        });

    } else {
        // This is a new player, so we need to create one.
        log::debug!("New Player created, set initial username to {}", ctx.sender);

        ctx.db.player_entity().insert(PlayerEntity {
            identity: ctx.sender,
            transform,
        });
    }
}


/*

pub fn is_crossing_chunk(ctx: &ReducerContext, x: f64, y: f64, z: f64) -> bool {
    if let Some(position) = ctx.db.position().identity().find(ctx.sender) {
        let dx = position.x - x;
        let dy = position.y - y;
        let dz = position.z - z;

        if dx.abs() > 1.0 || dy.abs() > 1.0 || dz.abs() > 1.0 {
            return true;
        }
    }
    false
}

pub fn get_position(ctx: &ReducerContext) -> Option<(f64, f64, f64)> {
    if let Some(position) = ctx.db.position().identity().find(ctx.sender) {
        Some((position.x, position.y, position.z))
    }
    else {
        None
    }
}

 */