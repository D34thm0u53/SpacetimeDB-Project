use spacetimedb::{table, ReducerContext};

use spacetimedsl::dsl;

use crate::modules::entity_position::*;
use crate::modules::entity_rotation::*;

/* 
Tables
- entity
*/

// Structure for the non-player entity table
#[dsl(plural_name = entities)]
#[table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    #[auto_inc]
    id: u64, // The rotation of the player.
    

}

/* 
Functions
- remove_player_from_entity_tables
*/

pub fn remove_player_from_entity_tables(ctx: &ReducerContext) {
let dsl = dsl(ctx);
    // Remove the entity position and chunk for the player
    dsl.delete_entity_position_by_player_identity(&ctx.sender);
    dsl.delete_entity_chunk_by_player_identity(&ctx.sender);
    dsl.delete_entity_rotation_by_player_identity(&ctx.sender);
    
}