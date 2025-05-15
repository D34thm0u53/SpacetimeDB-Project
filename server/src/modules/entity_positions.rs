use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};
use spacetimedb::SpacetimeType;

// Structure for the non-player entity table
#[spacetimedb::table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    #[auto_inc]
    id: u64, // The rotation of the player.
}

// Structure for the entity position table
#[spacetimedb::table(name = stdb_position, public)]
#[derive(Clone)]
pub struct StdbPosition {
    #[primary_key]
    player_id_fk: u64, // Fk to the player table
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(SpacetimeType, Debug, Clone, )]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: StdbPosition) {
    // The user has provided us with an update of their current position
    if let Some(player_entity_position) = ctx.db.stdb_position().player_id_fk().find(new_position.player_id_fk) {
        ctx.db.stdb_position().player_id_fk().update(
            StdbPosition {
                x: new_position.x,
                y: new_position.y,
                z: new_position.z,
                ..player_entity_position
            }
        );
    }
    else {
        ctx.db.stdb_position().insert( 
            StdbPosition {
                x: new_position.x,
                y: new_position.y,
                z: new_position.z,
                player_id_fk: new_position.player_id_fk,

            }
        );
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