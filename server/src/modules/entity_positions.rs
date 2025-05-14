use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};
use spacetimedb::SpacetimeType;

// Store current location of users
#[table(name = chunk, public)]
pub struct Chunk {
    #[primary_key]
    #[auto_inc]
    identity: u64,
    x: u32,
    y: u32,
    z: u32,
}

// Structure for the entity position table
#[spacetimedb::table(name = stdb_position, public)]
#[derive(Clone)]
pub struct StdbPosition {
    #[primary_key]
    identity: u64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
// Structure for the entity rotation table
#[spacetimedb::table(name = stdb_rotation, public)]
#[derive(Clone)]
pub struct StdbRotation {
    #[primary_key]
    identity: u64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Structure for the player entity table
#[table(name = player_entity, public)]
pub struct PlayerEntity {
    #[primary_key]
    #[auto_inc]
    player_id: u64,  // The identity of the player.

}

// Structure for the non-player entity table
#[spacetimedb::table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    identity: Identity,
    #[unique]
    position_fk: u64,     // The position of the player.
    #[unique]
    transform_fk: u64, // The rotation of the player.
}

// Structure for the transform data
#[derive(SpacetimeType, Debug, Clone,)]
pub struct Transform {
    pub position: Position,
    pub rotation: Rotation,
}

#[derive(SpacetimeType, Debug, Clone, )]
pub struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(SpacetimeType, Debug, Clone,)]
pub struct Rotation {
    x: f32,
    y: f32,
    z: f32,
}

#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, transform: Transform) {
    // The user has provided us with an update of their current position
    
    if let Some(player_entity) = ctx.db.player_entity().player_id().find(ctx.sender) {
        // Update the user's internal position
        let player_entity_position = ctx.db.stdb_position().identity().find(player_entity.player_id);

        ctx.db.stdb_position().identity().update( 
            StdbPosition {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            }
        );

        // Update the user's internal rotation
        ctx.db.stdb_rotation().identity().update( 
            StdbRotation {
                
                x: transform.rotation.x,
                y: transform.rotation.y,
                z: transform.rotation.z,
                ..stdb_rotation
            }
        );

    } else {
        // This is a new player, so we need to create one.

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