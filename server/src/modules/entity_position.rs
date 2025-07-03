use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::dsl;


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
    #[wrap]
    id: u64,
    pub player_identity: Identity, // Fk to the player table
    pub x: f32,
    pub y: f32,
    pub z: f32, // When the position was last modified
}

// Structure for the entity position table
#[dsl(plural_name = entity_chunks)]
#[table(name = entity_chunk, public)]
pub struct EntityChunk {
    #[primary_key]
    #[wrap]
    id: u64,
    pub player_identity: Identity, // Fk to the player table
    pub chunk_x: i32, // New: player's current chunk x
    pub chunk_z: i32, // New: player's current chunk z
    render_topleft_x: i32, // The top-left x chunk for the render area. used in RLS
    render_topleft_z: i32, // The top-left z chunk for the render area. used in RLS
    render_bottomright_x: i32, // The bottom-right x chunk for the render area. used in RLS
    render_bottomright_z: i32, // The bottom-right z chunk for the render area. used in RLS
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



/* 

*/
#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: EntityPosition) {
    // The user has provided us with an update of their current position
    let dsl = dsl(ctx);

    match dsl.get_entity_position_by_player_identity(&ctx.sender) {
        Some(mut entity_position) => {
            // If the position is the same, do not update
            // This is to prevent unnecessary writes to the database
            if (entity_position.x == new_position.x) && (entity_position.y == new_position.y) && (entity_position.z == new_position.z) {
                return;
            }
            else {
                entity_position.x = new_position.x;
                entity_position.y = new_position.y;
                entity_position.z = new_position.z;

                dsl.update_entity_position_by_player_identity(entity_position)
                    .expect("Failed to update entity position");
            }

        },
        None => {
            dsl.create_entity_position(
                ctx.sender,
                new_position.x,
                new_position.y,
                new_position.z,

            ).expect("Failed to create entity position");
        },
    }
}



#[spacetimedb::reducer]
pub fn update_my_position_by_delete(ctx: &ReducerContext, new_position: EntityPosition) {
    let dsl = dsl(ctx);
    // Delete any existing row for this player, returns bool
    let deleted = dsl.delete_entity_position_by_player_identity(&ctx.sender);
    if !deleted {
        // If not deleted, ensure there is no record for this player.
        let existing = dsl.get_entity_position_by_player_identity(&ctx.sender);
        assert!(existing.is_none(), "EntityPosition record for player_identity should not exist, but was found");
    }
    // Insert the new position row for this player
    dsl.create_entity_position(
        ctx.sender,
        new_position.x,
        new_position.y,
        new_position.z,
    ).expect("Failed to create entity position");
}
