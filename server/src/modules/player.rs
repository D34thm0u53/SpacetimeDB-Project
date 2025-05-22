use spacetimedb::{reducer, table, Identity, ReducerContext, Table, Timestamp};



use crate::modules::util::*;



// This module handles player connection events and player name management.
// It is responsible for creating player records in the database.

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
#[reducer]
pub fn handle_player_connection_event(ctx: &ReducerContext, event: i8 ) {
 

    let current_player = ctx.db.player().identity().find(ctx.sender)
        .unwrap_or_else(|| create_player(ctx));


}

//// private Fns ///

fn player_login(ctx: &ReducerContext ) {
    let dsl = dsl(ctx);
    // Get the player record from the database.
    // Create it if need be.
    let current_player = dsl.get_player_by_identity(&ctx.sender)
        .unwrap_or_else(|| create_player(ctx));

    

}

fn player_logout(ctx: &ReducerContext ) {
    let dsl = dsl(ctx);
    // Get the player record from the database.
    // Create it if need be.
    let mut current_player = dsl.get_player_by_identity(&ctx.sender)
        .unwrap_or_else(|| create_player(ctx));
    
    // Log the audit action
    log_player_action_audit(
        ctx,
        &format!("Player [{}] (Identity: [{}]) logged out", &current_player.username, &current_player.identity)
    );

    current_player.online = false;

    dsl
        .update_player_by_identity(
            current_player,
        );

}


fn create_player(ctx: &ReducerContext) -> Player {
    let dsl = dsl(ctx);

    // Prepare our needed data
    let username = ctx.sender.to_string();
    
    dsl
        .create_player(ctx.sender, &username, true)
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

pub fn get_username(ctx: &ReducerContext, identity: &Identity) -> String {
    let dsl = dsl(ctx);
    match dsl.get_player_by_identity(identity) {
        Some(player) => player.username.clone(),
        None => "".to_string(),
    }
}