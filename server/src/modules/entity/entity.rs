use spacetimedb::{table, ReducerContext, SpacetimeType };
use spacetimedsl::dsl;

//pub mod definitions; // Definitions for initial ingested data.


use super::entity_position::*;
use super::entity_rotation::*;

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
#[dsl(plural_name = entities,
    method(
        update = false,
        delete = true
    ),
    hook(after(insert))
)]
#[table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    #[index(btree)]
    #[auto_inc]
    #[use_wrapper(crate::modules::player::PlayerAccountId)]
    #[referenced_by(path = super::entity_position, table = entity_position)]
    #[referenced_by(path = super::entity_rotation, table = entity_rotation)]
    #[foreign_key(path = crate::modules::player, column = id, table = player_account, on_delete = Delete)]
    id: u32,

    entity_type: EntityType, // The type of the entity.

}


/// Hook that automatically creates related records after Entity insertion.
/// Creates EntityPosition, EntityRotation, and EntityChunk records with default values.
#[spacetimedsl::hook]
fn after_entity_insert(
    dsl: &spacetimedsl::DSL,
    new_entity: &Entity,
) -> Result<(), spacetimedsl::SpacetimeDSLError> {
    
    // Create the entity position record with default coordinates
    dsl.create_entity_position(CreateEntityPosition {
        id: new_entity.get_id(),
        x: 0,
        y: 0,
        z: 0,
    })?;
    
    // Create the entity rotation record with default rotation
    dsl.create_entity_rotation(CreateEntityRotation {
        id: new_entity.get_id(),
        rot_x: 0,
        rot_y: 0,
        rot_z: 0,
    })?;
    
    // Create the entity chunk record (starting at chunk 0,0)
    dsl.create_entity_chunk(CreateEntityChunk {
        id: new_entity.get_id(),
        chunk_x: 0,
        chunk_z: 0,
    })?;

    Ok(())
}


// Pub Fns

/// Creates a new entity with the specified type.
/// Automatically creates related EntityPosition, EntityRotation, and EntityChunk records
/// through the after_entity_insert hook.
pub fn create_entity_tree(ctx: &ReducerContext, entity_type: EntityType) -> Result<Entity, String> {
    let dsl = dsl(ctx);
    
    // Create a new entity - the hook will automatically create related records
    dsl.create_entity(CreateEntity {
        entity_type,
    })
        .map_err(|e| format!("Failed to create entity: {:?}", e))
}




