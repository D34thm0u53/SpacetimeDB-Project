use std::time::Duration;
use spacetimedb::*;
use spacetimedsl::*;

use crate::modules::player::*;
use crate::modules::entity::entity::*;
use crate::modules::entity::entity_position::*;
use crate::modules::util::{get_config_u64, CONFIG_CHUNK_UPDATE_INTERVAL_MS};


#[dsl(
    plural_name = scheduled_chunk_checks,
    method(
        update = false, 
        delete = true
    )
)]

#[spacetimedb::table(name = scheduled_chunk_check, scheduled(calculate_current_chunks))]
pub struct ScheduledChunkCheck {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]   
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}


/// Creates a chunk check timer if one doesn't already exist (runs at configured interval).
pub fn wrap_create_scheduled_chunk_check(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Check if a chunk check timer already exists
    let existing_timers: Vec<_> = dsl.get_all_scheduled_chunk_checks().collect();
    
    if !existing_timers.is_empty() {
        return Ok(());
    }
    // Once per configured interval, update player chunks based on their positions
    let interval_ms = get_config_u64(ctx, CONFIG_CHUNK_UPDATE_INTERVAL_MS).unwrap_or(5000);
    dsl.create_scheduled_chunk_check(CreateScheduledChunkCheck {
        scheduled_at: spacetimedb::ScheduleAt::Interval(Duration::from_millis(interval_ms).into()),
    })?;
    Ok(())
}

/// Reducer to calculate and update the current chunks for all online players' entities based on their positions.
#[spacetimedb::reducer]
pub fn calculate_current_chunks(ctx: &ReducerContext, _timer: ScheduledChunkCheck) -> Result<(), String> {
    let dsl = dsl(ctx);

    for player in dsl.get_all_online_players() {
        let player_id = player.get_id().value();

        // Find the entity owned by this player
        let entity = match dsl.get_entities_by_owner_id(&player_id).next() {
            Some(e) => e,
            None => continue, // Skip if player has no entity
        };

        let entity_id = entity.get_id();

        // Get the entity position
        let position = match dsl.get_entity_position_by_id(&entity_id) {
            Ok(pos) => pos,
            Err(_) => continue, // Skip if position not found
        };

        // Get or create the entity chunk
        let mut entity_chunk: EntityChunk = match dsl.get_entity_chunk_by_id(&entity_id) {
            Ok(chunk) => chunk,
            Err(_) => {
                // Create a new entity chunk if one doesn't exist
                dsl.create_entity_chunk(CreateEntityChunk {
                    id: entity_id.clone(),
                    chunk_x: (position.get_x() / 16) as u32,
                    chunk_z: (position.get_y() / 16) as u32,
                })?
            }
        };

        // Update chunk coordinates based on position
        entity_chunk.set_chunk_x((position.get_x() / 16) as u32);
        entity_chunk.set_chunk_z((position.get_y() / 16) as u32);

        dsl.update_entity_chunk_by_id(entity_chunk)?;
    }
    Ok(())
}
