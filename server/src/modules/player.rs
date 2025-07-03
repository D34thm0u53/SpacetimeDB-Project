use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::dsl;

use crate::modules::util::*;

// This module handles player connection events and player name management.
// It is responsible for creating player records in the database.

#[dsl(plural_name = online_players)]
#[table(name = online_player, public)]
pub struct OnlinePlayer {
    #[primary_key]
    #[auto_inc]
    #[wrap]    
    id: u64, // Auto-incremented ID for the player record
    pub identity: Identity,
    #[unique]
    pub username: String,
    created_at: Timestamp,
    modified_at: Timestamp,
}


#[dsl(plural_name = offline_players)]
#[table(name = offline_player, public)]
pub struct OfflinePlayer {
    #[primary_key]
    #[auto_inc]
    #[wrap]    
    id: u64,
    pub identity: Identity,
    #[unique]
    pub username: String,
    created_at: Timestamp,
    modified_at: Timestamp,
}



//// Impls ///



//// Reducers ///
 
#[spacetimedb::reducer]
pub fn set_username(ctx: &ReducerContext, username: String) {
let dsl = dsl(ctx);
    // Check if the username is valid
    match validate_name(username.clone()) {
        Ok(valid_username) => {
            // Check if the username already exists in the database
            if dsl.get_online_player_by_username(&valid_username).is_some() {
                log::warn!("Username [{}] already exists", valid_username);
                return;
            }
            // Update the player's username in the database

            let mut player = dsl
                .get_online_player_by_identity(&ctx.sender)
                .unwrap_or_else(|| create_player(ctx)); 

            player.username = valid_username.clone();

            log_player_action_audit(
                ctx,
                &format!(
                    "Player [{}] (Identity: [{}]) set username to [{}]",
                    &player.username, &player.identity, &valid_username
                ),
            );

            dsl.update_online_player_by_identity(player).expect("Failed to update player record");
            
        },
        Err(err) => {
            log::warn!("Invalid username: {}", err);
        }
    }


}

//// Public Fns ///

// Return player username by identity.
// Username str empty if not found.
pub fn get_username(ctx: &ReducerContext, identity: Identity) -> String {
    let dsl = dsl(ctx);
    match dsl.get_online_player_by_identity(&identity) {
        Some(player) => player.username,
        None => "".to_string(),
    }
}

pub fn handle_player_connection_event(ctx: &ReducerContext, event: u8 ) {
    match event {
        1 => { // Logon event
            move_player_to_online(ctx)
        },
        2 => { // Logoff event
            move_player_to_offline(ctx)
        },
        _ => {
            log::warn!("Unknown player connection event: {}", event);
        }
    }
    
}


//// private Fns ///

fn create_player(ctx: &ReducerContext) -> OnlinePlayer {
    let dsl = dsl(ctx);
    // Fetch a random username from the API
    let username = ctx.sender.to_string();
    dsl.create_online_player(ctx.sender, &username)
        .expect("Failed to create player record")

}

/// Takes a name and checks if it's acceptable as a user's name.
fn validate_name(username: String) -> Result<String, String> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if trimmed.len() > 32 {
        return Err("Username must be 32 characters or less".to_string());
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("Username contains invalid characters (allowed: a-z, A-Z, 0-9, _, -)".to_string());
    }
    Ok(trimmed.to_string())
}


fn move_player_to_offline(ctx: &ReducerContext) {
    let dsl = dsl(ctx);
    if let Some(player_record) = dsl.get_online_player_by_identity(&ctx.sender) {
                dsl.create_offline_player(player_record.identity, &player_record.username).expect("Failed to create offline player");
                dsl.delete_online_player_by_identity(&player_record.identity);
                
                log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged out", player_record.username, player_record.identity)
                );
            }
            else {
                // If the player is not found in online players, create a new player record
                log::warn!("Player identity [{}] reached logout while found in online players, creating new player record.", ctx.sender);
                let player_record = create_player(ctx);
                dsl.delete_online_player_by_identity(&player_record.identity);
                dsl.create_offline_player(player_record.identity, &player_record.username).expect("Failed to create offline player");
                log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged out.", player_record.username, player_record.identity)
                );
            }
}

fn move_player_to_online(ctx: &ReducerContext) {
    let dsl = dsl(ctx);
    if let Some(player_record) = dsl.get_offline_player_by_identity(&ctx.sender) {
                dsl.create_online_player(player_record.identity, &player_record.username).expect("Failed to create offline player");
                dsl.delete_offline_player_by_identity(&player_record.identity);
                
                log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged in", player_record.username, player_record.identity)
                );
            }
            else {
                let player_record = create_player(ctx);
                dsl.create_online_player(player_record.identity, &player_record.username).expect("Failed to create offline player");
                
                log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged in for the first time.", player_record.username, player_record.identity)
                );
            }
}
