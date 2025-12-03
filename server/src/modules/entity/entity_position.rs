use spacetimedb::*;
use spacetimedsl::*;

use super::entity::EntityId;
use super::entity::entity__view;

/// Position information for entities.
#[dsl(plural_name = entity_positions,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = entity_position, public)]
pub struct EntityPosition {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(super::entity::EntityId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Incoming position updates buffer table.
/// Stores position updates as they arrive from clients before batch processing.
#[dsl(plural_name = entity_positions_incoming,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = entity_position_incoming, public)]
pub struct EntityPositionIncoming {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u64,
    #[index(btree)]
    #[use_wrapper(super::entity::EntityId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    entity_id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    created_at: Timestamp,
}

/// Chunk information for entities.
#[dsl(plural_name = entity_chunks,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = entity_chunk,
    public,
    index(name = x, btree(columns = [chunk_x])),
    index(name = z, btree(columns = [chunk_z]))
)]
pub struct EntityChunk {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(super::entity::EntityId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    /// Entity's current chunk X coordinate
    pub chunk_x: u32,
    /// Entity's current chunk Z coordinate
    pub chunk_z: u32,
    /// When the chunk was last modified
    modified_at: Timestamp,
}


#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: EntityPosition) -> Result<(), String> {
    let dsl = dsl(ctx);

    let entity_id = EntityId::new(new_position.id);
    
    // Verify the entity exists before inserting into incoming buffer
    let _position_record: EntityPosition = dsl.get_entity_position_by_id(&entity_id)?;

    // Instead of updating directly, write to the incoming buffer table
    // The scheduler will process these and update the main table
    dsl.create_entity_position_incoming(CreateEntityPositionIncoming {
        entity_id: entity_id,
        x: new_position.x,
        y: new_position.y,
        z: new_position.z,
    })
    .map(|_| ())
    .map_err(|e| format!("Failed to buffer position update: {:?}", e))
}



use crate::modules::player::*;

/// View to get entity chunks within a 3-chunk radius of the viewer.
/// Finds the viewer's player-owned entity, then returns nearby chunks.
#[view(name = nearby_entity_chunks, public)]
pub fn nearby_entity_chunks(ctx: &ViewContext) -> Vec<EntityChunk> {
    log::debug!("nearby_entity_chunks called by identity: {}", ctx.sender);

    // Get the viewer's player account
    let viewer = match ctx.db.player_account().identity().find(&ctx.sender) {
        Some(v) => v,
        None => {
            return Vec::new();
        }
    };

    log::debug!("Viewer found: player_id={}", viewer.id);

    // Find the entity owned by this player
    let viewer_entity = match ctx.db.entity().owner_id().filter(&viewer.id).next() {
        Some(entity) => entity,
        None => {
            log::debug!("No entity found for player_id={}", viewer.id);
            return Vec::new();
        }
    };

    // Get viewer's current chunk position using the entity's ID
    let viewer_chunk = match ctx.db.entity_chunk().id().find(&viewer_entity.get_id().value()) {
        Some(chunk) => chunk,
        None => {
            return Vec::new();
        }
    };
    
    let viewer_chunk_x = viewer_chunk.chunk_x;
    let viewer_chunk_z = viewer_chunk.chunk_z;
    
    // Calculate the range bounds (within 3 chunks)
    let min_chunk_x = viewer_chunk_x.saturating_sub(3);
    let max_chunk_x = viewer_chunk_x.saturating_add(3);
    let min_chunk_z = viewer_chunk_z.saturating_sub(3);
    let max_chunk_z = viewer_chunk_z.saturating_add(3);
    
    // Use the btree indexes to filter chunks within range
    let mut nearby_chunks = Vec::new();
    
    // Filter by chunk_x range using the x index
    for chunk in ctx.db.entity_chunk().x().filter(min_chunk_x..=max_chunk_x) {
        // Further filter by chunk_z range
        if chunk.chunk_z >= min_chunk_z && chunk.chunk_z <= max_chunk_z {
            nearby_chunks.push(chunk);
        }
    }
    
    return nearby_chunks
}


