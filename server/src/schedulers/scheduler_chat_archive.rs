use std::{time::Duration};


use spacetimedb::{ReducerContext};
use spacetimedsl::{dsl};

use crate::modules::chat::*;



#[dsl(plural_name = chat_archive_timers)]
#[spacetimedb::table(name = chat_archive_timer, scheduled(archive_old_global_chat_messages))]
pub struct ChatArchiveTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
    current_update: u8,
}


pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx); // Waiting for DSL implementation of timers

    // Once per minute, check if we have over our target for global chat messages
    dsl.create_chat_archive_timer(spacetimedb::ScheduleAt::Interval(Duration::from_secs(60).into()), 0)?;
    Ok(())
}

#[spacetimedb::reducer]
pub fn archive_old_global_chat_messages(ctx: &ReducerContext, mut _timer: ChatArchiveTimer) -> Result<(), String> {
    let dsl = dsl(ctx);
    let mut messages: Vec<_> = dsl.get_all_global_chat_messages().collect();
    if messages.len() > 100 {
        messages.sort_by_key(|m| m.id); // Sort by sequential auto-increment id
        let to_archive = &messages[..messages.len() - 100];
        for msg in to_archive {
            dsl.create_global_chat_message_archive(
                msg.identity,
                &msg.username,
                &msg.message,
            )?;
            dsl.delete_global_chat_message_by_id(msg);
        }
    }
    Ok(())
}

