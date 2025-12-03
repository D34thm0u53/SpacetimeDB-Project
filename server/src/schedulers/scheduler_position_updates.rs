use std::time::Duration;
use std::collections::HashMap;
use spacetimedb::*;
use spacetimedsl::*;

use crate::modules::entity::entity_position::*;
use crate::modules::entity::entity::EntityId;
use crate::modules::util::{get_config_u64, CONFIG_POSITION_UPDATE_INTERVAL_MS};


#[dsl(
    plural_name = position_update_timers,
    method(
        update = false, 
        delete = true
    )
)]
#[spacetimedb::table(name = position_update_timer, scheduled(process_position_updates))]
pub struct PositionUpdateTimer {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]   
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}


/// Creates a position update timer if one doesn't already exist (runs at configured interval).
pub fn wrap_create_position_update_timer(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Check if a position update timer already exists
    let existing_timers: Vec<_> = dsl.get_all_position_update_timers().collect();
    
    if !existing_timers.is_empty() {
        return Ok(());
    }
    
    // Get the configured interval (default to 100ms for smooth updates)
    let interval_ms = get_config_u64(ctx, CONFIG_POSITION_UPDATE_INTERVAL_MS).unwrap_or(100);
    
    dsl.create_position_update_timer(CreatePositionUpdateTimer {
        scheduled_at: spacetimedb::ScheduleAt::Interval(Duration::from_millis(interval_ms).into()),
    })?;
    
    spacetimedb::log::info!("Position update timer created with interval: {}ms", interval_ms);
    Ok(())
}

/// Scheduled reducer that processes buffered position updates.
/// Only the most recent position update per entity is applied to the main table.
#[spacetimedb::reducer]
pub fn process_position_updates(ctx: &ReducerContext, _timer: PositionUpdateTimer) -> Result<(), String> {
    // Security check: Ensure only the scheduler can call this reducer
    if ctx.sender != ctx.identity() {
        return Err("Reducer process_position_updates may not be invoked by clients, only via scheduling.".to_string());
    }

    let dsl = dsl(ctx);
    
    // Collect all incoming position updates
    let incoming_updates: Vec<_> = dsl.get_all_entity_positions_incoming().collect();
    
    if incoming_updates.is_empty() {
        // No updates to process
        return Ok(());
    }
    
    spacetimedb::log::debug!("Processing {} buffered position updates", incoming_updates.len());
    
    // Group updates by entity_id and keep only the most recent (highest id = latest received)
    let mut latest_updates: HashMap<EntityId, EntityPositionIncoming> = HashMap::new();
    
    for update in incoming_updates {
        let entity_id = update.get_entity_id().clone();
        
        // Keep the update with the highest id (most recent) for each entity
        latest_updates.entry(entity_id)
            .and_modify(|existing| {
                if update.get_id().value() > existing.get_id().value() {
                    *existing = update.clone();
                }
            })
            .or_insert(update);
    }
    
    let mut updated_count = 0;
    let mut deleted_count = 0;
    let mut failed_count = 0;
    
    // Apply the latest update for each entity to the main position table
    for (entity_id, latest_update) in latest_updates.iter() {
        // Get the current position record
        match dsl.get_entity_position_by_id(entity_id) {
            Ok(mut position_record) => {
                // Check if position has actually changed to avoid unnecessary updates
                if position_record.x != latest_update.x
                    || position_record.y != latest_update.y
                    || position_record.z != latest_update.z {
                    
                    // Update the position
                    position_record.x = latest_update.x;
                    position_record.y = latest_update.y;
                    position_record.z = latest_update.z;
                    
                    match dsl.update_entity_position_by_id(position_record) {
                        Ok(_) => {
                            updated_count += 1;
                        }
                        Err(e) => {
                            spacetimedb::log::warn!(
                                "Failed to update position for entity {}: {:?}", 
                                entity_id.value(), 
                                e
                            );
                            failed_count += 1;
                        }
                    }
                }
            }
            Err(e) => {
                spacetimedb::log::warn!(
                    "Entity {} not found when processing position update: {:?}", 
                    entity_id.value(), 
                    e
                );
                failed_count += 1;
            }
        }
    }
    
    // Clean up all processed incoming records
    let all_incoming: Vec<_> = dsl.get_all_entity_positions_incoming().collect();
    for incoming in all_incoming {
        match dsl.delete_entity_position_incoming_by_id(&incoming.get_id()) {
            Ok(_) => deleted_count += 1,
            Err(e) => {
                spacetimedb::log::warn!(
                    "Failed to delete incoming position record {}: {:?}", 
                    incoming.get_id().value(), 
                    e
                );
            }
        }
    }
    
    if updated_count > 0 || deleted_count > 0 {
        spacetimedb::log::debug!(
            "Position update batch complete: {} updated, {} records deleted, {} failed",
            updated_count,
            deleted_count,
            failed_count
        );
    }
    
    Ok(())
}
