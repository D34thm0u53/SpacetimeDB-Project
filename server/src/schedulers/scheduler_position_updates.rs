use std::time::Duration;
use std::collections::HashMap;
use spacetimedb::*;
use spacetimedsl::*;

use crate::modules::entity::entity_position::*;
use crate::modules::entity::entity::EntityId;
use crate::modules::util::{get_config_u64, CONFIG_POSITION_UPDATE_INTERVAL_MS};


#[dsl(
    plural_name = scheduled_position_updates,
    method(
        update = false, 
        delete = true
    )
)]
#[spacetimedb::table(name = scheduled_position_update, scheduled(process_position_updates))]
pub struct ScheduledPositionUpdate {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]   
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}


/// Creates a position update timer if one doesn't already exist (runs at configured interval).
pub fn wrap_create_scheduled_position_update(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Check if a position update timer already exists
    let existing_timers: Vec<_> = dsl.get_all_scheduled_position_updates().collect();
    
    if !existing_timers.is_empty() {
        return Ok(());
    }
    
    // Get the configured interval (default to 50ms for smooth updates)
    let interval_ms = get_config_u64(ctx, CONFIG_POSITION_UPDATE_INTERVAL_MS).unwrap_or(50);
    
    dsl.create_scheduled_position_update(CreateScheduledPositionUpdate {
        scheduled_at: spacetimedb::ScheduleAt::Interval(Duration::from_millis(interval_ms).into()),
    })?;
    
    spacetimedb::log::info!("Position update timer created with interval: {}ms", interval_ms);
    Ok(())
}

/// Scheduled reducer that processes buffered position updates.
/// Only the most recent position update per entity is applied to the main table.
#[spacetimedb::reducer]
pub fn process_position_updates(ctx: &ReducerContext, _timer: ScheduledPositionUpdate) -> Result<(), String> {
    // Security check: Ensure only the scheduler can call this reducer
    if ctx.sender != ctx.identity() {
        return Err("Reducer process_position_updates may not be invoked by clients, only via scheduling.".to_string());
    }

    let dsl = dsl(ctx);
    
    // Collect all incoming position updates
    let incoming_updates: Vec<_> = dsl.get_all_entity_positions_incoming().collect();
    
    if incoming_updates.is_empty() {
        return Ok(());
    }
    
    // Track unique entities and their latest update (by highest auto_inc ID)
    // Key: entity_id as u32, Value: the update record (auto_inc ID used for comparison)
    let mut latest_updates: HashMap<u32, EntityPositionIncoming> = 
        HashMap::with_capacity(incoming_updates.len() / 2 + 1);
    
    // Single pass: find latest update per entity using auto_inc ID for ordering
    for update in incoming_updates {
        let entity_id = update.get_entity_id().value();
        
        latest_updates
            .entry(entity_id)
            .and_modify(|existing| {
                // Higher auto_inc ID = more recent update
                if update.get_id().value() > existing.get_id().value() {
                    *existing = update.clone();
                }
            })
            .or_insert(update);
    }
    
    // Process and apply updates in a single pass
    for (entity_id_raw, latest_update) in &latest_updates {
        let entity_id = EntityId::new(*entity_id_raw);
        
        if let Ok(mut position_record) = dsl.get_entity_position_by_id(&entity_id) {
            // Only update if position actually changed
            if position_record.x != latest_update.x
                || position_record.y != latest_update.y
                || position_record.z != latest_update.z
            {
                position_record.x = latest_update.x;
                position_record.y = latest_update.y;
                position_record.z = latest_update.z;
                
                let _ = dsl.update_entity_position_by_id(position_record);
            }
        }
        
        // Delete all incoming records for this entity using the btree index
        // This is more efficient than deleting by primary key one at a time
        let _ = dsl.delete_entity_positions_incoming_by_entity_id(&entity_id);
    }
    
    Ok(())
}
