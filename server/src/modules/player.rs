use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::dsl;

use crate::modules::util::*;



// This module handles player connection events and player name management.
// It is responsible for creating player records in the database.

#[dsl(plural_name = players)]
#[table(name = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    #[unique]
    pub username: String,
    pub online: bool,
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
            if dsl.get_player_by_username(&valid_username).is_some() {
                log::warn!("Username [{}] already exists", valid_username);
                return;
            }
            // Update the player's username in the database
            let player = ctx.db.player().identity().find(ctx.sender)
                .unwrap_or_else(|| create_player(ctx));
            ctx.db.player().identity().update(
                Player {
                    username: valid_username,
                    ..player
                }
            );
        },
        Err(err) => {
            log::warn!("Invalid username: {}", err);
        }
    }


}

//// Public Fns ///

// Return player username by identity.
// Username str empty if not found.
pub fn get_username(ctx: &ReducerContext) -> String {
    let dsl = dsl(ctx);
    match dsl.get_player_by_identity(&ctx.sender) {
        Some(player) => player.username,
        None => "".to_string(),
    }
}

pub fn handle_player_connection_event(ctx: &ReducerContext, event: u8 ) {
    let dsl = dsl(ctx);
    let mut current_player = dsl
        .get_player_by_identity(&ctx.sender)
        .unwrap_or_else(|| create_player(ctx));

    match event {
        1 => {
            current_player.online = true;
            log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged in", current_player.username, current_player.identity)
            );
        },
        2 => {
            current_player.online = false;
            log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged out", current_player.username, current_player.identity)
            );
        },
        _ => {
            log::warn!("Unknown player connection event: {}", event);
        }
    }
    dsl.update_player_by_identity(current_player).expect("Failed to update player record");
}


//// private Fns ///

fn create_player(ctx: &ReducerContext) -> Player {
    let dsl = dsl(ctx);
    // Fetch a random username from the API
    let username = ctx.sender.to_string();
    dsl.create_player(ctx.sender, &username, true)
        .expect("Failed to create player record");
    ctx.db.player().identity().find(ctx.sender).unwrap()
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
    // Uniqueness check: must not already exist in the player table
    // (Assumes access to ctx is available; if not, pass ctx as an argument)
    // This function signature does not have ctx, so uniqueness must be checked in the reducer.
    Ok(trimmed.to_string())
}

