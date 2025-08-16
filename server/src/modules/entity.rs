use spacetimedb::{table, ReducerContext, SpacetimeType };
use spacetimedsl::{ dsl, Wrapper };

//pub mod definitions; // Definitions for initial ingested data.

pub mod reducers; // SpacetimeDB Reducers for this file's structs.


use crate::modules::entity_position::*;
use crate::modules::entity_rotation::*;

/* 
Tables
- entity
*/

/// What kind of entity it is.
#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq, PartialOrd)] pub enum EntityType {
    Player,
    NonPlayer,
    World
}


// Structure for the entity table
#[dsl(plural_name = entities)]
#[table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    #[auto_inc]
    #[use_wrapper(path = crate::modules::player::PlayerAccountId)]
    #[referenced_by(path = crate::modules::entity_position, table = entity_position)]
    #[referenced_by(path = crate::modules::entity_rotation, table = entity_rotation)]
    #[foreign_key(path = crate::modules::player, column = id, table = player_account, on_delete = Delete)]
    id: u32,

    entity_type: EntityType, // The type of the entity.

}


// Pub Fns
pub fn create_entity_tree(ctx: &ReducerContext, entity_type: EntityType) -> Entity {
    let dsl = dsl(ctx);
    // Create a new entity

    let entity = dsl.create_entity(entity_type)
        .expect("Failed to create entity");

    // Create the entity position and rotation records
    dsl.create_entity_position(
        crate::modules::player::PlayerAccountId::new(entity.id),
        0,
        0,
        0
    ).expect("Failed to create entity position");
    dsl.create_entity_rotation(
        crate::modules::player::PlayerAccountId::new(entity.id),
        0,
        0,
        0
    ).expect("Failed to create entity rotation");

    return entity
}




