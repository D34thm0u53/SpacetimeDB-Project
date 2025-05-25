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
pub struct EnityPosition {
    #[primary_key]
    player_identity: Identity, // Fk to the player table
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub chunk_x: i32, // New: player's current chunk x
    pub chunk_z: i32, // New: player's current chunk z
    render_topleft_x: i32, // The top-left x chunk for the render area. used in RLS
    render_topleft_z: i32, // The top-left z chunk for the render area. used in RLS
    render_bottomright_x: i32, // The bottom-right x chunk for the render area. used in RLS
    render_bottomright_z: i32, // The bottom-right z chunk for the render area. used in RLS
    modified_at: Timestamp, // When the position was last modified
}

#[derive(SpacetimeType)]
pub struct StdbPosition {
    pub player_identity: Identity,
    pub x: f32,
    pub y: f32,
    pub z: f32,

}

use spacetimedb::{client_visibility_filter, Filter};
///You can only see ship objects in your sector.
#[client_visibility_filter]
const SO_SECTOR_FILTER: Filter = Filter::Sql(
    //JOIN entity_position sender_ep ON ep.player_identity = sender_ep.player_identity
"
    SELECT others_ep.*
    FROM entity_position others_ep
    JOIN entity_position ep
    WHERE (
            others_ep.chunk_x >= ep.render_topleft_x AND
            ep.chunk_z >= others_ep.render_topleft_z AND 
            others_ep.chunk_z <= ep.render_bottomright_x AND
            ep.chunk_z <= others_ep.render_bottomright_z AND 
            ep.player_identity = :sender)
");


#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: StdbPosition) {
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

                let chunk_x = (new_position.x / 50.0).floor() as i32;
                let chunk_z = (new_position.z / 50.0).floor() as i32;

                entity_position.x = new_position.x;
                entity_position.y = new_position.y;
                entity_position.z = new_position.z;
                // Update chunk_x and chunk_z as well
                entity_position.chunk_x = chunk_x;
                entity_position.chunk_z = chunk_z;
                entity_position.render_topleft_x = chunk_x -5;
                entity_position.render_topleft_z = chunk_z -5;
                entity_position.render_bottomright_x = chunk_x +5;
                entity_position.render_bottomright_z = chunk_z +5;

                dsl.update_entity_position_by_player_identity(entity_position)
                    .expect("Failed to update entity position");
            }

        },
        None => {
            let chunk_x = (new_position.x / 50.0).floor() as i32;
            let chunk_z = (new_position.z / 50.0).floor() as i32;

            dsl.create_entity_position(
                ctx.sender,
                new_position.x,
                new_position.y,
                new_position.z,
                chunk_x,
                chunk_z,
                chunk_x -5,
                chunk_z -5,
                chunk_x +5,
                chunk_z +5   
            ).expect("Failed to create entity position");
        },
    }
}

