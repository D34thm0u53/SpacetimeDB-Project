use std::{time::Duration};
use spacetimedb::{table, ReducerContext, reducer};
use spacetimedsl::*;

use crate::modules::chat::*;
use crate::modules::util::{get_config_u64, CONFIG_CHAT_MESSAGE_LIMIT};



#[dsl(
    plural_name = scheduled_chat_archives,
    method(
        update = false, 
        delete = true
    )
)]
#[table(
    name = scheduled_chat_archive,
    scheduled(
        archive_old_global_chat_messages
    )
)]
pub struct ScheduledChatArchive {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]    
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
    current_update: u8,
}


/// Creates a chat archive timer if one doesn't already exist (runs every 60 seconds).
pub fn wrap_create_scheduled_chat_archive(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Check if a chat archive timer already exists
    let existing_timers: Vec<_> = dsl.get_all_scheduled_chat_archives().collect();
    
    if !existing_timers.is_empty() {
        return Ok(());
    }

    // Once per minute, check if we have over our target for global chat messages
    dsl.create_scheduled_chat_archive(CreateScheduledChatArchive {
        scheduled_at: spacetimedb::ScheduleAt::Interval(Duration::from_secs(60).into()),
        current_update: 0,
    })?;
    Ok(())
}

/// Scheduled reducer that archives global chat messages exceeding the configured limit.
#[reducer]
pub fn archive_old_global_chat_messages(ctx: &ReducerContext, mut _timer: ScheduledChatArchive) -> Result<(), String> {
    // Security check: Ensure only the scheduler can call this reducer
    if ctx.sender != ctx.identity() {
        return Err("Reducer archive_old_global_chat_messages may not be invoked by clients, only via scheduling.".to_string());
    }

    let dsl = dsl(ctx);
    
    // Get the configurable message limit (defaults to 100 if not set)
    let message_limit = get_config_u64(ctx, CONFIG_CHAT_MESSAGE_LIMIT).unwrap_or(100) as usize;
    
    // Get all global chat messages
    let mut all_messages: Vec<_> = dsl.get_all_global_chat_messages().collect();
    
    // Sort by created_at timestamp (oldest first)
    all_messages.sort_by(|a, b| a.get_created_at().cmp(&b.get_created_at()));

    // If we have message_limit or fewer messages, no archiving needed
    if all_messages.len() <= message_limit {
        spacetimedb::log::debug!(
            "Chat archive check: {} messages (limit: {}), no archiving needed",
            all_messages.len(),
            message_limit
        );
        return Ok(());
    }

    // Calculate how many messages to archive (keep only the latest message_limit)
    let messages_to_archive = all_messages.len() - message_limit;
    let messages_to_move = &all_messages[0..messages_to_archive];
    
    let mut archived_count = 0;
    let mut failed_count = 0;
    
    // Archive old messages by moving them to the archive table
    for message in messages_to_move {
        // Create archive entry (created_at will be set automatically by DSL)
        match dsl.create_global_chat_message_archive(CreateGlobalChatMessageArchive {
            identity: *message.get_identity(),
            username: message.get_username().clone(),
            message: message.get_message().clone(),
        }) {
            Ok(_) => {
                // Successfully archived, now delete from main table
                match dsl.delete_global_chat_message_by_id(&message.get_id()) {
                    Ok(_) => {
                        archived_count += 1;
                    }
                    Err(e) => {
                        spacetimedb::log::warn!("Failed to delete message with ID {} after archiving: {:?}", message.get_id(), e);
                        failed_count += 1;
                    }
                }
            }
            Err(e) => {
                spacetimedb::log::error!("Failed to archive message with ID {}: {:?}", message.get_id(), e);
                failed_count += 1;
            }
        }
    }
    
    if archived_count > 0 {
        spacetimedb::log::info!("Successfully archived {} global chat messages", archived_count);
    }
    
    if failed_count > 0 {
        spacetimedb::log::warn!("Failed to archive {} global chat messages", failed_count);
        return Err(format!("Failed to archive {} messages", failed_count));
    }
    
    Ok(())
}

