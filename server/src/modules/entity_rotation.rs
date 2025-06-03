use spacetimedb::{table, Identity, ReducerContext};

use spacetimedsl::dsl;

/* 
Tables
- entity_rotation

*/

// Structure for the entity position table
#[dsl(plural_name = entity_rotations)]
#[table(name = entity_rotation, public)]
pub struct EntityRotation {
    #[primary_key]
    player_identity: Identity, // Fk to the player table
    pub rot_x: i16,
    pub rot_y: i16,
    pub rot_z: i16,
}

/* 
Recducers
- update_my_rotation: Updates the rotation of the player in the entity_rotation table. This data is used in a scheduled task to update the player's chunk position in the entity_chunk table.

*/

#[spacetimedb::reducer]
pub fn update_my_rotation(ctx: &ReducerContext, new_rotation: EntityRotation) {
    // The user has provided us with an update of their current position
    let dsl = dsl(ctx);

    match dsl.get_entity_rotation_by_player_identity(&ctx.sender) {
        Some(mut entity_rotation) => {
            // If the position is the same, still update to refresh the modified_at timestamp
            // This is useful for keeping the position updated without changing it
            if (entity_rotation.rot_x == new_rotation.rot_x) && (entity_rotation.rot_y == new_rotation.rot_y) && (entity_rotation.rot_z == new_rotation.rot_z) {
                dsl.update_entity_rotation_by_player_identity(entity_rotation)
                    .expect("Failed to update entity rotation");
                return;
            }
            else {
                entity_rotation.rot_x = new_rotation.rot_x;
                entity_rotation.rot_y = new_rotation.rot_y;
                entity_rotation.rot_z = new_rotation.rot_z;

                dsl.update_entity_rotation_by_player_identity(entity_rotation)
                    .expect("Failed to update entity rotation");
            }

        },
        None => {
            dsl.create_entity_rotation(
                ctx.sender,
                new_rotation.rot_x,
                new_rotation.rot_y,
                new_rotation.rot_z,

            ).expect("Failed to create entity rotation");
        },
    }
}
