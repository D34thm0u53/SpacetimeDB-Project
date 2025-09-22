use spacetimedb::{table, ReducerContext, Timestamp };
use spacetimedsl::{ dsl };

use super::entity::*;


/* 
Tables:

- entity_position: Stores the position of entities (players) in the game world.
- entity_chunk: Stores the chunk information for entities (players) in the game world.
*/
// Structure for the entity position table


#[dsl(plural_name = entity_positions)]
#[table(name = entity_position, public)]
pub struct EntityPosition {
    #[primary_key]
    #[use_wrapper(path = crate::modules::player::PlayerAccountId)]
    #[foreign_key(path = crate::modules::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32, // When the position was last modified
}

// Structure for the entity position table
#[dsl(plural_name = entity_chunks)]
#[table(name = entity_chunk, public)]
pub struct EntityChunk {
    #[primary_key]
    #[create_wrapper]
    id: u32,
    pub chunk_x: u32, // New: player's current chunk x
    pub chunk_z: u32, // New: player's current chunk z
    render_topleft_x: u32, // The top-left x chunk for the render area. used in RLS
    render_topleft_z: u32, // The top-left z chunk for the render area. used in RLS
    render_bottomright_x: u32, // The bottom-right x chunk for the render area. used in RLS
    render_bottomright_z: u32, // The bottom-right z chunk for the render area. used in RLS
    modified_at: Timestamp, // When the position was last modified
}






/* 
Reducers
## Note: 
    Due to SpacetimeDB's design, performing a row update is actually a delete followed by an insert.
    At a later date with load testing, we will find which method is more performant.
    1. Load the record, compare if it matches the new position, and update if it does not.
    2. Delete the record, then insert the new position. regardless of data matching.

- update_my_position: Updates the position of the player in the entity_position table.
    # Performs a check to see if the position has changed.


- update_my_position_by_delete: Deletes the existing position of the player in the entity_position table and creates a new position.
    # This is useful for ensuring that the position is always updated, even if it is the same as before.

*/






#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, entity: Entity, new_position: EntityPosition) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Verify the entity exists
    match dsl.get_entity_by_id(entity.get_id()) {
        Ok(_) => {
            // Entity exists, proceed with position update
            match dsl.get_entity_position_by_id(entity.get_id()) {
                Ok(current_position) => {
                    // Check if position has actually changed to avoid unnecessary updates
                    if current_position.x == new_position.x
                        && current_position.y == new_position.y
                        && current_position.z == new_position.z {
                        return Ok(());
                    }

                    // Update the existing position
                    let updated_position = EntityPosition {
                        x: new_position.x,
                        y: new_position.y,
                        z: new_position.z,
                        ..current_position // Preserve other fields like ID
                    };

                    dsl.update_entity_position_by_id(updated_position)
                        .map_err(|e| format!("Failed to update entity position: {:?}", e))?;
                }
                Err(spacetimedsl::SpacetimeDSLError::NotFoundError { .. }) => {
                    // Entity exists but has no position record - create one
                    dsl.create_entity_position(entity.get_id(), new_position.x, new_position.y, new_position.z)
                        .map_err(|e| format!("Failed to create entity position: {:?}", e))?;
                }
                Err(e) => {
                    return Err(format!("Failed to retrieve entity position: {:?}", e));
                }
            }
        }
        Err(_) => {
            log::info!("Entity not found for position update: {:?}", entity.get_id());
            return Err("Entity not found".to_string());
        }
    }

    Ok(())
}




