use spacetimedb::{table, ReducerContext };
use spacetimedsl::{ dsl };


use crate::modules::entity::entity::*;

/* 
Tables
- entity_rotation

*/

// Structure for the entity position table
#[dsl(plural_name = entity_rotations, method(update = true, delete = true))]
#[table(name = entity_rotation, public)]
pub struct EntityRotation {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(crate::modules::player::PlayerAccountId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    pub rot_x: i16,
    pub rot_y: i16,
    pub rot_z: i16,
}

/* 
Recducers
- update_my_rotation: Updates the rotation of the player in the entity_rotation table.
    # Performs a check to see if the rotation has changed.
    # If it has not changed, it will still update the record to refresh the modified_at timestamp.

*/

#[spacetimedb::reducer]
pub fn update_my_rotation(ctx: &ReducerContext, entity: Entity, new_rotation: EntityRotation) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Verify the entity exists
    match dsl.get_entity_by_id(entity.get_id()) {
        Ok(_) => {
            // Entity exists, proceed with rotation update
            match dsl.get_entity_rotation_by_id(entity.get_id()) {
                Ok(current_rotation) => {
                    // Check if rotation has actually changed to avoid unnecessary updates
                    if current_rotation.rot_x == new_rotation.rot_x 
                        && current_rotation.rot_y == new_rotation.rot_y 
                        && current_rotation.rot_z == new_rotation.rot_z {
                        return Ok(());
                    }

                    // Update the existing rotation
                    let updated_rotation = EntityRotation {
                        rot_x: new_rotation.rot_x,
                        rot_y: new_rotation.rot_y,
                        rot_z: new_rotation.rot_z,
                        ..current_rotation // Preserve other fields like ID
                    };

                    dsl.update_entity_rotation_by_id(updated_rotation)
                        .map_err(|e| format!("Failed to update entity rotation: {:?}", e))?;
                }
                Err(spacetimedsl::SpacetimeDSLError::NotFoundError { .. }) => {
                    // Entity exists but has no rotation record - create one
                    dsl.create_entity_rotation(CreateEntityRotation {
                        id: entity.get_id(),
                        rot_x: new_rotation.rot_x,
                        rot_y: new_rotation.rot_y,
                        rot_z: new_rotation.rot_z,
                    })
                        .map_err(|e| format!("Failed to create entity rotation: {:?}", e))?;
                }
                Err(e) => {
                    return Err(format!("Failed to retrieve entity rotation: {:?}", e));
                }
            }
        }
        Err(_) => {
            log::info!("Entity not found for rotation update: {:?}", entity.get_id());
            return Err("Entity not found".to_string());
        }
    }

    Ok(())
}
