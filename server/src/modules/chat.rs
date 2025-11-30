use spacetimedb::{table, Identity, ReducerContext, Timestamp};
use spacetimedsl::*;

use crate::modules::player::*;

#[dsl(plural_name = global_chat_messages,
    method(
        update = true
    )
)]
#[table(name = global_chat_message, public)]
pub struct GlobalChatMessage {
    #[primary_key]
    #[index(btree)]
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
#[dsl(plural_name = direct_messages,
    method(
        update = true
    )
)]
#[table(name = direct_message, public, index(name = sender_and_receiver, btree(columns = [sender_id, receiver_id])))]
pub struct DirectMessage {
    #[primary_key]
    #[index(btree)]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
    #[index(btree)]
    #[use_wrapper(PlayerAccountId)]
    #[foreign_key(path = crate, column = id, table = player_account, on_delete = SetZero)]
    pub sender_id: u32, // FK to Player
    #[index(btree)]
    #[use_wrapper(PlayerAccountId)]
    #[foreign_key(path = crate, column = id, table = player_account, on_delete = SetZero)]
    pub receiver_id: u32, // FK to Player
    pub message: String,
    pub sent_at: Timestamp,
}





#[dsl(plural_name = player_ignore_pairs,
    unique_index(
        name = ignorer_and_ignored
    ),
    method(
        update = true
    )
)]
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


#[dsl(plural_name = global_mute_lists,
    method(
        update = true
    )
)]
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
#[dsl(plural_name = global_chat_message_archives,
    method(
        update = true
    )
)]
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


// Filter to only show Private messages, where the receiver is the client.
#[client_visibility_filter]
const PRIVATE_CHAT_MESSAGE_RECEIVER_FILTER: Filter = Filter::Sql(
    "SELECT dm.*
    FROM direct_message dm
    JOIN player_account ON dm.sender_id = player_account.id
    WHERE dm.receiver_id = player_account.id
    AND player_account.identity = :sender",
);

// Filter to only show Private messages, where the sender is the client.
#[client_visibility_filter]
const PRIVATE_CHAT_MESSAGE_SENDER_FILTER: Filter = Filter::Sql(
    "SELECT dm.*
    FROM direct_message dm
    JOIN player_account ON dm.sender_id = player_account.id
    WHERE dm.sender_id = player_account.id
    AND player_account.identity = :sender",
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
            dsl.create_player_ignore_pair(CreatePlayerIgnorePair {
                ignorer_identity: ctx.sender,
                ignored_identity: target_identity,
            })
                .map_err(|e| format!("Failed to create ignore relationship: {:?}", e))?;
            
            log::debug!("Player {} ignored player {}", ctx.sender, target_identity);
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

            log::debug!("Player {} unignored player {}", ctx.sender, target_identity);
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
/// Sends a message to the global chat channel.
#[spacetimedb::reducer]
pub fn send_global_chat(ctx: &ReducerContext, message: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    
    dsl.create_global_chat_message(CreateGlobalChatMessage {
        identity: ctx.sender,
        username: get_username_by_identity(ctx, ctx.sender),
        message,
    })?;
    Ok(())
}



/// Sends a private message to a player identified by username.
#[spacetimedb::reducer]
pub fn send_private_chat(ctx: &ReducerContext, target_username: String, message: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Look up the receiver's identity by username
    // get the PlayerAccount by username
    let receiver_account = dsl.get_player_account_by_username(&target_username)?;
    let sender_account = dsl.get_player_account_by_identity(&ctx.sender)?;

    dsl.create_direct_message(CreateDirectMessage {
        sender_id: sender_account.get_id(),
        receiver_id: receiver_account.get_id(),
        message,
        sent_at: ctx.timestamp,
    })?;
    Ok(())
}
