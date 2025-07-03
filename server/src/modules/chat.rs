use spacetimedb::{table, Identity, ReducerContext, Timestamp};
use spacetimedsl::dsl;

use crate::modules::player::*;
use crate::modules::common::*;
use crate::modules::util::log_player_action_audit;

#[dsl(plural_name = global_chat_messages)]
#[table(name = global_chat_message, public)]
pub struct GlobalChatMessage {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    id: u64,

    pub identity: Identity, // FK to Player
    pub username: String,
    pub message: String,

    created_at: Timestamp,
}

/// Table for storing private messages sent between players.
/// Each row represents a single message from sender to receiver, with content and timestamp.
#[dsl(plural_name = private_chat_messages)]
#[table(name = private_chat_message, public, index(name = sender_and_receiver, btree(columns = [sender_identity, receiver_identity])))]
pub struct PrivateChatMessage {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    id: u64,
    pub sender_identity: Identity, // FK to Player
    pub receiver_identity: Identity, // FK to Player
    pub message: String,
    pub sent_at: Timestamp,
}



#[dsl(plural_name = player_ignore_pairs, unique_index(name = ignorer_and_ignored))]
#[table(name = player_ignore_pair, public, index(name = ignorer_and_ignored, btree(columns = [ignorer_identity, ignored_identity])))]
pub struct PlayerIgnorePair {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    id: u64,
    pub ignorer_identity: Identity, // FK to Player
    pub ignored_identity: Identity, // FK to Player
    created_at: Timestamp,
}


#[dsl(plural_name = global_mute_lists)]
#[table(name = global_mute_list, public)]
pub struct GlobalMuteList {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    id: u64,
    #[unique]
    pub identity: Identity, // FK to Player
    reason: String,
    created_at: Timestamp,
    modified_at: Timestamp,
    expires_at: Option<Timestamp>, // Optional expiration time for the mute
}

/// Archive table for global chat messages.
/// Stores all messages purged from the main global_chat_message table.
#[dsl(plural_name = global_chat_message_archives)]
#[table(name = global_chat_message_archive, public)]
pub struct GlobalChatMessageArchive {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    id: u64,
    pub identity: Identity,
    pub username: String,
    pub message: String,
    created_at: Timestamp, // Remove pub for inherited visibility
}


use spacetimedb::{client_visibility_filter, Filter};
#[client_visibility_filter]
const PLAYER_IGNORE_PAIR_FILTER: Filter = Filter::Sql(
    "SELECT * FROM player_ignore_pair WHERE ignorer_identity = :sender",
);
#[client_visibility_filter]
const PRIVATE_CHAT_MESSAGE_FILTER: Filter = Filter::Sql(
    "SELECT * FROM private_chat_message WHERE receiver_identity = :sender",
);



#[spacetimedb::reducer]
pub fn ignore_target_player(ctx: &ReducerContext, username_to_block: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Check if the target player is already ignored
    if let Some(target_identity) = get_player_identity_by_username(ctx, &username_to_block) {
        // Check if the ignore pair already exists
        if dsl.get_player_ignore_pair_by_ignorer_and_ignored(&ctx.sender, &target_identity).is_some() {
            return Err("Player is already ignored.".to_string());
        }
        else {
            match dsl.create_player_ignore_pair(ctx.sender, target_identity) {
                        Ok(_) => {
                            log::debug!("created ignore pair for {} and {}", ctx.sender, target_identity);
                        }
                        Err(_) => {
                            log::warn!("Failed to create ignore pair for {} and {}", ctx.sender, target_identity);
                            return Err("Failed to create ignore pair. Please try again.".to_string());
                        }
                    }
        }
    log_player_action_audit(ctx, &format!("ignore:{}", target_identity));
    Ok(())
    }
    else {
        Err("Target player not found.".to_string())
    }
    
}

#[spacetimedb::reducer]
pub fn unignore_target_player(ctx: &ReducerContext, username_to_block: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Check if the target player is actually ignored
    if let Some(target_identity) = get_player_identity_by_username(ctx, &username_to_block) {
        // If the target player is not ignored, return an error
        if let Some(ignore_pair_record) = dsl.get_player_ignore_pair_by_ignorer_and_ignored(&ctx.sender, &target_identity) {
            dsl.delete_player_ignore_pair_by_id(&ignore_pair_record.id);
        }
        else {
            return Err("Player is not ignored.".to_string());
        }
        log_player_action_audit(ctx, &format!("unignore:{}", target_identity));
        Ok(())
    }
    else {
        return Err("Target player not found.".to_string());
    }
    
}

//// Reducers ///
#[spacetimedb::reducer]
pub fn send_global_chat(ctx: &ReducerContext, chat_message: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Check if the sender is muted globally
    if dsl.get_global_mute_list_by_identity(&ctx.sender).is_some() {
        return Err("You are globally muted and cannot send messages.".to_string());
    }
    else {
        dsl.create_global_chat_message(ctx.sender, &get_username(ctx, ctx.sender), &chat_message)?;
        Ok(())
    }

}



/// Reducer to send a private message from the sender to a target player by username.
/// The message is always saved, even if the receiver is ignoring the sender (for audit purposes).
#[spacetimedb::reducer]
pub fn send_private_chat(ctx: &ReducerContext, target_username: String, message: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Look up the receiver's identity by username
    let Some(receiver_identity) = get_player_identity_by_username(ctx, &target_username) else {
        return Err("Target player not found".to_string());
    };
    // Save the message regardless of ignore status
    dsl.create_private_chat_message(ctx.sender, receiver_identity, &message, ctx.timestamp)?;
    Ok(())
}



