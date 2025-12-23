use std::time::Duration;
use std::collections::HashMap;
use spacetimedb::*;
use spacetimedsl::*;

use crate::modules::entity::entity_rotation::*;
use crate::modules::entity::entity::EntityId;
use crate::modules::util::{get_config_u64, CONFIG_ROTATION_UPDATE_INTERVAL_MS};


#[dsl(
    plural_name = scheduled_rotation_updates,
    method(
        update = false, 
        delete = true
    )
)]
#[spacetimedb::table(name = scheduled_rotation_update, scheduled(process_rotation_updates))]
pub struct ScheduledRotationUpdate {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]   
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}


/// Creates a rotation update timer if one doesn't already exist (runs at configured interval).
pub fn wrap_create_scheduled_rotation_update(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Check if a rotation update timer already exists
    let existing_timers: Vec<_> = dsl.get_all_scheduled_rotation_updates().collect();
    
    if !existing_timers.is_empty() {
        return Ok(());
    }
    
    // Get the configured interval (default to 200ms for ~5 Hz with client-side interpolation)
    let interval_ms = get_config_u64(ctx, CONFIG_ROTATION_UPDATE_INTERVAL_MS).unwrap_or(200);
    
    dsl.create_scheduled_rotation_update(CreateScheduledRotationUpdate {
        scheduled_at: spacetimedb::ScheduleAt::Interval(Duration::from_millis(interval_ms).into()),
    })?;

    spacetimedb::log::info!("Rotation update timer created with interval: {}ms", interval_ms);
    Ok(())
}

/// Scheduled reducer that processes buffered rotation updates.
/// Only the most recent rotation update per entity is applied to the main table.
#[spacetimedb::reducer]
pub fn process_rotation_updates(ctx: &ReducerContext, _timer: ScheduledRotationUpdate) -> Result<(), String> {
    // Security check: Ensure only the scheduler can call this reducer
    if ctx.sender != ctx.identity() {
        return Err("Reducer process_rotation_updates may not be invoked by clients, only via scheduling.".to_string());
    }

    let dsl = dsl(ctx);
    
    // Collect all incoming rotation updates
    let incoming_updates: Vec<_> = dsl.get_all_entity_rotations_incoming().collect();
    
    if incoming_updates.is_empty() {
        return Ok(());
    }
    
    // Track unique entities and their latest update (by highest auto_inc ID)
    // Key: entity_id as u32, Value: the update record (auto_inc ID used for ordering)
    let mut latest_updates: HashMap<u32, EntityRotationIncoming> = 
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
        
        if let Ok(mut rotation_record) = dsl.get_entity_rotation_by_id(&entity_id) {
            // Only update if rotation actually changed
            if rotation_record.rot_x != latest_update.rot_x
                || rotation_record.rot_y != latest_update.rot_y
                || rotation_record.rot_z != latest_update.rot_z
            {
                rotation_record.rot_x = latest_update.rot_x;
                rotation_record.rot_y = latest_update.rot_y;
                rotation_record.rot_z = latest_update.rot_z;
                
                if let Err(e) = dsl.update_entity_rotation_by_id(rotation_record) {
                    log::warn!("Failed to update rotation for entity {}: {:?}", entity_id_raw, e);
                }
            }
        } else {
            // Entity rotation record doesn't exist, create one
            if let Err(e) = dsl.create_entity_rotation(CreateEntityRotation {
                id: entity_id.clone(),
                rot_x: latest_update.rot_x,
                rot_y: latest_update.rot_y,
                rot_z: latest_update.rot_z,
            }) {
                log::warn!("Failed to create rotation for entity {}: {:?}", entity_id_raw, e);
            }
        }
        
        // Delete all incoming records for this entity using the btree index
        // This is more efficient than deleting by primary key one at a time
        
        if let Err(e) = dsl.delete_entity_rotations_incoming_by_entity_id(&entity_id) {
                log::warn!("Failed to delete incoming rotations for entity {}: {:?}", entity_id_raw, e);
        }
        
    }
    
    Ok(())
}
