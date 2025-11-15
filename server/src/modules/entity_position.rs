use spacetimedb::{table, ReducerContext, Timestamp };
use spacetimedsl::Wrapper;
use spacetimedsl::{ dsl };


/* 
Tables:

- entity_position: Stores the position of entities (players) in the game world.
- entity_chunk: Stores the chunk information for entities (players) in the game world.
*/
// Structure for the entity position table


#[dsl(plural_name = entity_positions, method(update = true, delete = true))]
#[table(name = entity_position, public)]
pub struct EntityPosition {
    #[primary_key]
    #[use_wrapper(crate::modules::player::PlayerAccountId)]
    #[foreign_key(path = crate::modules::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32, // When the position was last modified
}

// Structure for the entity position table
#[dsl(plural_name = entity_chunks, method(update = true, delete = true))]
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
    Due to SpacetimeDB's atomic design, performing a row update is actually a delete followed by an insert.
    At a later date with load testing, we will find which method is more performant.
    1. Load the record, compare if it matches the new position, and update if it does not.
    2. Delete the record, then insert the new position. regardless of data matching.

- update_my_position: Updates the position of the player in the entity_position table.
    # Performs a check to see if the position has changed.


- update_my_position_by_delete: Deletes the existing position of the player in the entity_position table and creates a new position.
    # This is useful for ensuring that the position is always updated, even if it is the same as before.

*/
#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: EntityPosition) -> Result<(), String> {
    let dsl = dsl(ctx);

    use crate::modules::player::PlayerAccountId;
    let player_id = PlayerAccountId::new(new_position.id);
    let mut position_record = dsl.get_entity_position_by_id(player_id)?;

    // Check if position has actually changed to avoid unnecessary updates
    if position_record.x == new_position.x
        && position_record.y == new_position.y
        && position_record.z == new_position.z {
        return Ok(());
    }

    // Update the existing position
    position_record.x = new_position.x;
    position_record.y = new_position.y;
    position_record.z = new_position.z;
    dsl.update_entity_position_by_id(position_record)
        .map_err(|e| format!("Failed to update entity position: {:?}", e))?;
    Ok(())
}
