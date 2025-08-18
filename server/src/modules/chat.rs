use spacetimedb::{table, Identity, ReducerContext, Timestamp};
use spacetimedsl::dsl;

use crate::modules::player::*;
use crate::modules::common::*;

#[dsl(plural_name = global_chat_messages)]
#[table(name = global_chat_message, public)]
pub struct GlobalChatMessage {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
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
    #[create_wrapper]
    id: u32,
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
    #[create_wrapper]
    id: u32,
    pub ignorer_identity: Identity, // FK to Player
    pub ignored_identity: Identity, // FK to Player
    created_at: Timestamp,
}


#[dsl(plural_name = global_mute_lists)]
#[table(name = global_mute_list, public)]
pub struct GlobalMuteList {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
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
    #[create_wrapper]
    id: u32,
    pub identity: Identity,
    pub username: String,
    pub message: String,
    created_at: Timestamp,
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
pub fn ignore_player(ctx: &ReducerContext, target_identity: Identity) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Validate target identity
    if ctx.sender == target_identity {
        return Err("No matter how hard you try, you cannot ignore yourself".to_string());
    }

    // Check if ignore relationship already exists
    match dsl.get_player_ignore_pair_by_ignorer_and_ignored(&ctx.sender, &target_identity) {
        Ok(_existing_pair) => {
            return Err("Player is already ignored".to_string());
        }
        Err(spacetimedsl::SpacetimeDSLError::NotFoundError { .. }) => {
            // No existing ignore relationship, proceed to create one
            dsl.create_player_ignore_pair(ctx.sender, target_identity)
                .map_err(|e| format!("Failed to create ignore relationship: {:?}", e))?;
            
            log::info!("Player {} ignored player {}", ctx.sender, target_identity);
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to lookup ignore pair: {:?}", e))
        }
    }
}



#[spacetimedb::reducer]
pub fn unignore_player(ctx: &ReducerContext, target_identity: Identity) -> Result<(), String> {
    let dsl = dsl(ctx);

    // Validate target identity
    if ctx.sender == target_identity {
        return Err("Cannot unignore yourself".to_string());
    }

    // Check if ignore relationship already exists
    match dsl.get_player_ignore_pair_by_ignorer_and_ignored(&ctx.sender, &target_identity) {
        Ok(_existing_pair) => {
            dsl.delete_player_ignore_pair_by_ignorer_and_ignored(&ctx.sender, &target_identity)
                .map_err(|e| format!("Failed to delete ignore relationship: {:?}", e))?;

            log::info!("Player {} unignored player {}", ctx.sender, target_identity);
            Ok(())
        }
        Err(spacetimedsl::SpacetimeDSLError::NotFoundError { .. }) => {
            // No existing ignore relationship, proceed to create one
            return Err("Player is not ignored".to_string());
            
        }
        Err(e) => {
            Err(format!("Failed to lookup ignore pair: {:?}", e))
        }
    }
}


//// Reducers ///
#[spacetimedb::reducer]
pub fn send_global_chat(ctx: &ReducerContext, chat_message: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Check if the sender is muted globally
    // if dsl.get_global_mute_list_by_identity(&ctx.sender).is_some() {
    //     return Err("You are globally muted and cannot send messages.".to_string());
    // }
    // else 
        dsl.create_global_chat_message(ctx.sender, &get_username_by_identity(ctx, ctx.sender), &chat_message)?;
        Ok(())


}



/// Reducer to send a private message from the sender to a target player by username.
/// The message is always saved, even if the receiver is ignoring the sender (for audit purposes).
#[spacetimedb::reducer]
pub fn send_private_chat(ctx: &ReducerContext, target_username: String, message: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Look up the receiver's identity by username
    let Some(receiver_identity) = get_player_identity_by_username(ctx, &target_username) else {
        log::warn!("Failed to send private message: Target player '{}' not found", target_username);
        return Err("Target player not found".to_string());
    };
    // Save the message regardless of ignore status
    dsl.create_private_chat_message(ctx.sender, receiver_identity, &message, ctx.timestamp)?;
    Ok(())
}



