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
    player_identity: Identity, // Fk to the player table
    pub x: f32,
    pub y: f32,
    pub z: f32, // When the position was last modified
}

// Structure for the entity position table
#[dsl(plural_name = entity_chunks)]
#[table(name = entity_chunk, public)]
pub struct EntityChunk {
    #[primary_key]
    player_identity: Identity, // Fk to the player table
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

- update_my_position: Updates the position of the player in the entity_position table. This data is used in a scheduled task to update the player's chunk position in the entity_chunk table.
*/


#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: EntityPosition) {
    // The user has provided us with an update of their current position
    let dsl = dsl(ctx);

    match dsl.get_entity_position_by_player_identity(&ctx.sender) {
        Some(mut entity_position) => {
            // If the position is the same, still update to refresh the modified_at timestamp
            // This is useful for keeping the position updated without changing it
            if (entity_position.x == new_position.x) && (entity_position.y == new_position.y) && (entity_position.z == new_position.z) {
                dsl.update_entity_position_by_player_identity(entity_position)
                    .expect("Failed to update entity position");
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
