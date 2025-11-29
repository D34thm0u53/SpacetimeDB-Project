use std::{time::Duration};
use spacetimedb::*;
use spacetimedsl::*;

use crate::modules::player::*;
use crate::modules::entity::entity_position::*;
use crate::modules::util::{get_config_u64, CONFIG_CHUNK_UPDATE_INTERVAL_MS};


#[dsl(
    plural_name = chunk_check_timers,
    method(
        update = false, 
        delete = true
    )
)]

#[spacetimedb::table(name = chunk_check_timer, scheduled(calculate_current_chunks))]
pub struct ChunkCheckTimer {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]   
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
    current_update: u8,
}


pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Get the configurable chunk update interval (defaults to 5000ms if not set)
    let interval_ms = get_config_u64(ctx, CONFIG_CHUNK_UPDATE_INTERVAL_MS).unwrap_or(5000);
    
    spacetimedb::log::info!("Initializing chunk update scheduler with interval: {}ms", interval_ms);

    dsl.create_chunk_check_timer(CreateChunkCheckTimer {
        scheduled_at: spacetimedb::ScheduleAt::Interval(Duration::from_millis(interval_ms).into()),
        current_update: 0,
    })?;
    Ok(())
}

/// Reducer to calculate and update the current chunks for all online players entities based on their positions
#[spacetimedb::reducer]
pub fn calculate_current_chunks(ctx: &ReducerContext, mut _timer: ChunkCheckTimer) -> Result<(), String> {
    let dsl = dsl(ctx);


    for player in dsl.get_all_online_players() {

            // Convert the field we want.
            use crate::modules::player::PlayerAccountId;
            let player_id = PlayerAccountId::from(&player.get_id());

            // Get the entity position for the player
            let entity = match dsl.get_entity_position_by_id(&player_id) {
                Ok(e) => e,
                Err(_) => continue, // Skip if entity not found
            };

            // Get the entity chunk for the player
            let mut entity_chunk: EntityChunk = match dsl.get_entity_chunk_by_id(&entity.get_id()) {
                Ok(chunk) => chunk,
                Err(_) => {
                // Create a new entity chunk if one doesn't exist
                dsl.create_entity_chunk(CreateEntityChunk {
                    id: entity.get_id().clone(),
                    chunk_x: (entity.get_x() / 16) as u32,
                    chunk_z: (entity.get_y() / 16) as u32,
                })?
                }
            };

            entity_chunk.set_chunk_x((entity.get_x() / 16) as u32);
            entity_chunk.set_chunk_z((entity.get_y() / 16) as u32);

            dsl.update_entity_chunk_by_id(entity_chunk)?;
    }   
    Ok(())
}
