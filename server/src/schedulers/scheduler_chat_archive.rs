use std::{time::Duration};
use spacetimedb::{ReducerContext};
use spacetimedsl::{dsl};

use crate::modules::chat::*;



#[dsl(plural_name = chat_archive_timers)]
#[spacetimedb::table(name = chat_archive_timer, scheduled(archive_old_global_chat_messages))]
pub struct ChatArchiveTimer {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]    
    id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
    current_update: u8,
}


pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx); // Waiting for DSL implementation of timers

    // Once per minute, check if we have over our target for global chat messages
    dsl.create_chat_archive_timer(
        spacetimedb::ScheduleAt::Interval(Duration::from_secs(60).into()),
        0,
    )?;
    Ok(())
}

#[spacetimedb::reducer]
pub fn archive_old_global_chat_messages(ctx: &ReducerContext, mut _timer: ChatArchiveTimer) -> Result<(), String> {
    // Security check: Ensure only the scheduler can call this reducer
    if ctx.sender != ctx.identity() {
        return Err("Reducer archive_old_global_chat_messages may not be invoked by clients, only via scheduling.".to_string());
    }

    let dsl = dsl(ctx);
    
    // Get all global chat messages
    let mut all_messages: Vec<_> = dsl.get_all_global_chat_messages().collect();
    
    // Sort by created_at timestamp (oldest first)
    all_messages.sort_by(|a, b| a.get_created_at().cmp(&b.get_created_at()));
    
    // If we have 10 or fewer messages, no archiving needed
    if all_messages.len() <= 10 {
        return Ok(());
    }
    
    // Calculate how many messages to archive (keep only the latest 10)
    let messages_to_archive = all_messages.len() - 10;
    let messages_to_move = &all_messages[0..messages_to_archive];
    
    let mut archived_count = 0;
    let mut failed_count = 0;
    
    // Archive old messages by moving them to the archive table
    for message in messages_to_move {
        // Create archive entry (created_at will be set automatically by DSL)
        match dsl.create_global_chat_message_archive(
            *message.get_identity(),
            &message.get_username(),
            &message.get_message(),
        ) {
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

