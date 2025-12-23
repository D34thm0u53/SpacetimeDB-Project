use spacetimedb::{table, ReducerContext, Timestamp};
use spacetimedsl::dsl;
use spacetimedsl::*;

use super::entity::EntityId;

/// Rotation information for entities.
#[dsl(plural_name = entity_rotations,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = entity_rotation, public)]
pub struct EntityRotation {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(super::entity::EntityId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    pub rot_x: i16,
    pub rot_y: i16,
    pub rot_z: i16,
}

/// Incoming rotation updates buffer table.
/// Stores rotation updates as they arrive from clients before batch processing.
/// Scheduled to process at ~5 Hz (200ms interval) for client-side interpolation.
#[dsl(plural_name = entity_rotations_incoming,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = entity_rotation_incoming, public)]
pub struct EntityRotationIncoming {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u64,
    #[index(btree)]
    #[use_wrapper(super::entity::EntityId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    entity_id: u32,
    pub rot_x: i16,
    pub rot_y: i16,
    pub rot_z: i16,
    created_at: Timestamp,
}

/// Buffers a rotation update for the client's entity.
/// Instead of updating directly, writes to the incoming buffer table.
/// The scheduler processes these updates in batches and applies only the latest rotation per entity.
#[spacetimedb::reducer]
pub fn update_my_rotation(ctx: &ReducerContext, new_rotation: EntityRotation) -> Result<(), String> {
    let dsl = dsl(ctx);

    let entity_id = EntityId::new(new_rotation.id);
    
    // Verify the entity exists before buffering the rotation update
    let _rotation_record: EntityRotation = dsl.get_entity_rotation_by_id(&entity_id)?;

    // Write to the incoming buffer table.
    // The scheduler will process these and update the main table.
    dsl.create_entity_rotation_incoming(CreateEntityRotationIncoming {
        entity_id,
        rot_x: new_rotation.rot_x,
        rot_y: new_rotation.rot_y,
        rot_z: new_rotation.rot_z,
    })
    .map(|_| ())
    .map_err(|e| format!("Failed to buffer rotation update: {:?}", e))
}
