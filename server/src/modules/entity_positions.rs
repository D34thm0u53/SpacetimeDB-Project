use spacetimedb::{table, Identity, ReducerContext, Timestamp, SpacetimeType};

use spacetimedsl::dsl;

// Structure for the non-player entity table
#[dsl(plural_name = entities)]
#[table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    #[auto_inc]
    id: u64, // The rotation of the player.
}

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
#[derive(SpacetimeType)]
pub struct StdbPosition {
    pub player_identity: Identity,
    pub x: f32,
    pub y: f32,
    pub z: f32,

}
*/

/* 
use spacetimedb::{client_visibility_filter, Filter};
///You can only see ship objects in your sector.
#[client_visibility_filter]
const ENTITY_CHUNK_FILTER: Filter = Filter::Sql(
    "
    SELECT others_ec.*
    FROM entity_chunk others_ec
    JOIN entity_chunk my_ec
    WHERE (
            others_ec.chunk_x >= my_ec.render_topleft_x AND
            my_ec.chunk_z >= others_ec.render_topleft_z AND 
            others_ec.chunk_z <= my_ec.render_bottomright_x AND
            my_ec.chunk_z <= others_ec.render_bottomright_z AND 
            my_ec.player_identity = :sender)
    "
);



///You can only see ship objects in your sector.
#[client_visibility_filter]
const ENTITY_POSITION_FILTER: Filter = Filter::Sql(
    //JOIN entity_position sender_ep ON ep.player_identity = sender_ep.player_identity
"
    -- Select others entity positions that are within the render area of the sender's entity position
    SELECT entity_position.*
    FROM entity_position

    -- we need to join on entity chunk and entity position
    -- to get entity positions that are within the render area of the sender's entity position

    JOIN entity_chunk ec ON entity_position.player_identity = ec.player_identity
       
");
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

