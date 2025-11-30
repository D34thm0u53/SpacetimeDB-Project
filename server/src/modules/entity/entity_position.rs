use spacetimedb::*;
use spacetimedsl::*;

/// Position information for entities (players).
#[dsl(plural_name = entity_positions,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = entity_position)]
pub struct EntityPosition {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(crate::modules::player::PlayerAccountId)]
    #[foreign_key(path = super::entity, table = entity, column = id, on_delete = Delete)]
    id: u32,
    pub x: i32,
    pub y: i32,
    pub z: i32
}

/// Chunk information for entities (players).
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
    #[use_wrapper(crate::modules::player::PlayerAccountId)]
    id: u32,
    pub chunk_x: u32, // New: player's current chunk x
    
    pub chunk_z: u32, // New: player's current chunk z
    modified_at: Timestamp // When the position was last modified
}


#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, new_position: EntityPosition) -> Result<(), String> {
    let dsl = dsl(ctx);

    use crate::modules::player::PlayerAccountId;
    let player_id = PlayerAccountId::new(new_position.id);
    let mut position_record: EntityPosition = dsl.get_entity_position_by_id(player_id)?;

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



use crate::modules::player::*;
// View to get entity chunks within a 3-chunk radius of the viewer
#[view(name = nearby_entity_chunks, public)]
pub fn nearby_entity_chunks(ctx: &ViewContext) -> Vec<EntityChunk> {


    // Get the viewer's position first to determine their chunk
    let viewer = ctx.db.player_account().identity().find(&ctx.sender)
        .unwrap_or_else(|| panic!("Viewer not found in player account table"));
    
    // Get viewer's current chunk position
    let viewer_chunk = ctx.db.entity_chunk()
        .id()
        .find(&viewer.id)
        .unwrap_or_else(|| panic!("Viewer not found in chunk table"));
    
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



