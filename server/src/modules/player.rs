use spacetimedb::{reducer, table, Identity, ReducerContext, Table, Timestamp};

use crate::modules::uuid::StUuid;
use crate::modules::util::*;

// This module handles player connection events and player name management.
// It is responsible for creating player records in the database.
#[table(name = player, public)]
#[derive(Clone)]
pub struct Player {
    #[primary_key]
    pub uuid: StUuid,
    #[unique]
    pub identity: Identity,
    pub online: bool,
    pub last_seen: Timestamp,
    #[unique]
    pub username: String,
}


#[reducer]
pub fn handle_player_connection_event(ctx: &ReducerContext, event: i8 ) {
    match event {
        1 => player_login(ctx),
        2 => player_logout(ctx),
        _ => log::warn!("Unknown player connection event: {}", event),
    }

}

fn player_login(ctx: &ReducerContext ) {
    // Get the player record from the database. Created it if need be.
    let current_player = get_player(ctx);
    let uuid = current_player.uuid.clone();
    log_player_action_audit(
        ctx,
        uuid.clone(),
        format!("Player [{}] (UUID: [{}]) logged in", current_player.username, uuid)
    );
    ctx.db.player().uuid().update(Player { online: true, last_seen: ctx.timestamp, ..current_player });
}

fn get_player(ctx: &ReducerContext) -> Player {
    // Check if the player already exists in the database
    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Player already exists, update their online status
        return player;
    }
    else {
        create_player(ctx)
    }
    
}

fn create_player(ctx: &ReducerContext) -> Player {
    // Prepare our needed data
    let allocated_uuid = StUuid::new(ctx);
    let username = allocated_uuid.to_string().to_uppercase();
    
    // This is a new player, create a new entry in the database
    ctx.db.player().insert(Player {
        uuid: allocated_uuid,
        identity: ctx.sender,
        online: false,
        last_seen: ctx.timestamp,
        username: username
, // Default username
    });
    ctx.db.player().identity().find(ctx.sender).unwrap()
}


fn player_logout(ctx: &ReducerContext ) {
    // Get the player record from the database. Create it if need be.
    // Log the audit action
    // Update the player record

    let current_player = get_player(ctx);
    let uuid = current_player.uuid.clone();
    log_player_action_audit(
        ctx,
        uuid.clone(),
        format!("Player [{}] (UUID: [{}]) logged out", current_player.username, uuid)
    );
    ctx.db.player().uuid().update(Player { online: false, last_seen: ctx.timestamp, ..current_player });
}


// Name Management
#[reducer]
/// Clients invoke this reducer to set their user names.
fn set_user_name(ctx: &ReducerContext, username: String) -> Result<(), String> {
    let username = username.trim().to_string();
    let username = validate_name(username)?;
    if let Some(user) = ctx.db.player().identity().find(ctx.sender) {
        log::debug!("User {:?} requested update username to: {}", ctx.sender, username);
        ctx.db.player().identity().update(Player { username, ..user });
        Ok(())
    }
    else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to set username without connecting first.
        Err("Cannot set name for unknown user".to_string())
    }
}


/// Takes a name and checks if it's acceptable as a user's name.
fn validate_name(username: String) -> Result<String, String> {
    if username.len() > 64 {
        Err("Names must be less than 64 characters".to_string())
    }
    else if username.contains(' ') {
        Err("Names must not contain spaces".to_string())
    }
    else if username.contains('\n') {
        Err("Names must not contain newlines".to_string())
    }
    else if username.contains('\r') {
        Err("Names must not contain carriage returns".to_string())
    }
    else if username.contains('\0') {
        Err("Names must not contain null characters".to_string())
    }
    else if username.contains('\t') {
        Err("Names must not contain tabs".to_string())
    }
    else if username.contains('!') {
        Err("Names must not contain exclamation marks".to_string())
    }
    else if username.contains('@') {
        Err("Names must not contain at signs".to_string())
    }
    else if username.contains('#') {
        Err("Names must not contain hash signs".to_string())
    }
    else if username.contains('$') {
        Err("Names must not contain dollar signs".to_string())
    }
    else if username.contains('%') {
        Err("Names must not contain percent signs".to_string())
    }
    else if username.contains('^') {
        Err("Names must not contain caret signs".to_string())
    }
    else if username.contains('&') {
        Err("Names must not contain ampersands".to_string())
    }
    else if username.contains('*') {
        Err("Names must not contain asterisks".to_string())
    }
    
    else {
        Ok(username)
    }
}