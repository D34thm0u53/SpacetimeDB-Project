use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::{ dsl, Wrapper };
use log::*;
use crate::modules::util::*;

// player_account table is a persistent storage for player data.

#[dsl(plural_name = player_accounts)]
#[table(name = player_account, public)]
pub struct PlayerAccount {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    #[referenced_by(path = crate, table = online_player)]
    #[referenced_by(path = crate, table = offline_player)]
    #[referenced_by(path = crate::modules::roles, table = role)]
    id: u32, // Auto-incremented ID for the player record
    #[unique]
    pub identity: Identity,
    #[unique]
    pub username: String,
    created_at: Timestamp,
    modified_at: Timestamp,
}



// online_player is a table that stores currently online players.
#[dsl(plural_name = online_players)]
#[table(name = online_player, public)]
pub struct OnlinePlayer {
    #[primary_key]
    #[use_wrapper(name = PlayerAccountId)]
    #[foreign_key(path = crate, column = id, table = player_account, on_delete = Delete)]
    id: u32,
    #[unique]
    pub identity: Identity,
    created_at: Timestamp,
}

// offline_player is a table that stores currently offline players.
#[dsl(plural_name = offline_players)]
#[table(name = offline_player, public)]
pub struct OfflinePlayer {
   #[primary_key]
    #[use_wrapper(name = PlayerAccountId)]
    #[foreign_key(path = crate, column = id, table = player_account, on_delete = Delete)]

    id: u32,
    #[unique]
    pub identity: Identity,
    created_at: Timestamp,
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
            if dsl.get_player_account_by_username(&valid_username).is_ok() {
                log::warn!("Username [{}] already exists", valid_username);
                return;
            }
            // Find or create the player's account by identity
            let mut player = match dsl.get_player_account_by_identity(&ctx.sender) {
                Ok(acc) => acc,
                Err(_) => {
                    match create_player_account_and_online(ctx, ctx.sender, valid_username.clone()) {
                        Ok((acc, _online)) => acc,
                        Err(e) => {
                            log::error!("Failed to create PlayerAccount: {}", e);
                            return;
                        }
                    }
                }
            };

            // Update username using generated setter
            player.set_username(&valid_username);

            log_player_action_audit(
                ctx,
                &format!(
                    "Player [{}] (Identity: [{}]) set username to [{}]",
                    player.get_id(),
                    player.get_identity(),
                    valid_username
                ),
            );

            // Persist the change to the PlayerAccount
            dsl.update_player_account_by_identity(player).expect("Failed to update player record");
        },
        Err(err) => {
            log::warn!("Invalid username: {}", err);
        }
    }
}




//// Public Fns ///

// Return player username by identity.
// Returns the username for a given PlayerAccountsId, or empty string if not found.
pub fn get_username_by_id(ctx: &ReducerContext, id: PlayerAccountId) -> String {
    let dsl = dsl(ctx);
    match dsl.get_player_account_by_id(&id) {
        Ok(account) => account.get_username().to_string(),
        Err(_) => String::new(),
    }
}

// Returns the username for a given Identity, or empty string if not found.
pub fn get_username_by_identity(ctx: &ReducerContext, identity: Identity) -> String {
    let dsl = dsl(ctx);
    match dsl.get_player_account_by_identity(&identity) {
        Ok(account) => account.get_username().to_string(),
        Err(_) => String::new(),
    }
}


// Returns the Identity for a given username, or empty string if not found.
pub fn get_identity_by_username(ctx: &ReducerContext, username: String) -> Identity {
    let dsl = dsl(ctx);
    match dsl.get_player_account_by_username(&username) {
        Ok(account) => account.identity,
        Err(_) => Identity::default(),
    }
}


pub fn handle_player_connection_event(ctx: &ReducerContext, event: u8 ) {
    log::info!("Handling event [{}] for player: [{}]", event, ctx.sender);
    match event {
        1 => { // Logon event
            log::info!("Player [{}] logged in", ctx.sender);
            
            if !does_player_account_exist(ctx) {
                // Create a new player account if it doesn't exist
                let default_username: String = ctx.sender.to_string().chars().take(28).collect();
                match create_player_account_and_online(ctx, ctx.sender, default_username) {
                    Ok((player_account, online_player)) => {
                        log::info!("Created new PlayerAccount: {:?}", player_account);
                        log::info!("Created new OnlinePlayer: {:?}", online_player);
                    },
                    Err(e) => {
                        log::error!("Failed to create player account: {}", e);
                        return;
                    }
                }
            } else {
                log::info!("Player account already exists for identity [{}]", ctx.sender);
                move_player_to_online(ctx)
            }
            log::info!("Player [{}] moved to online.", ctx.sender);
            
        },
        2 => { // Logoff event
            move_player_to_offline(ctx)
        },
        _ => {
            log::warn!("Unknown player connection event: {}", event);
        }
    }
    
}


pub fn does_player_account_exist(ctx: &ReducerContext) -> bool {
    let dsl = dsl(ctx);
    dsl.get_player_account_by_identity(&ctx.sender).is_ok()
}

//// private Fns ///


/// Creates a new PlayerAccount and OnlinePlayer record for the given identity and username.
/// Returns Result<(PlayerAccount, OnlinePlayer), String> on success, or error message.
pub fn create_player_account_and_online(ctx: &ReducerContext, identity: Identity, username: String) -> Result<(PlayerAccount, OnlinePlayer), String> {
    let dsl = dsl(ctx);

    // Validate username
    let username = validate_name(username)?;

    // Check if identity or username already exists
    if dsl.get_player_account_by_identity(&identity).is_ok() {
        return Err("PlayerAccount for this identity already exists".to_string());
    }
    if dsl.get_player_account_by_username(&username).is_ok() {
        return Err("Username already taken".to_string());
    }
    log::info!("Creating PlayerAccount for identity [{}] with username [{}]", identity, username);
    // Create PlayerAccount
    let player_account = dsl.create_player_account(identity.clone(), &username)
        .map_err(|e| format!("Failed to create PlayerAccount: {:?}", e))?;

    log::info!("Created PlayerAccount: {:?}", player_account);
    // Create OnlinePlayer record
    let online_player = dsl.create_online_player(player_account.get_id(), player_account.identity)
        .map_err(|e| format!("Failed to create OnlinePlayer: {:?}", e))?;

    Ok((player_account, online_player))
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
    let online_player: Result<OnlinePlayer, spacetimedsl::SpacetimeDSLError> = dsl.get_online_player_by_identity(&ctx.sender);

    if online_player.is_err() {
        log::warn!("Player identity [{}] is not online", ctx.sender);
        return;
    }
    else {
        let player_record = online_player.unwrap();
        dsl.create_offline_player(player_record.get_id(), player_record.identity).expect("Failed to create offline player");
        dsl.delete_online_player_by_id(player_record.get_id());

        log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged out", player_record.get_id(), player_record.identity)
                );
    }


}

fn move_player_to_online(ctx: &ReducerContext) {
    let dsl = dsl(ctx);
    let offline_player = dsl.get_offline_player_by_identity(&ctx.sender);

    if offline_player.is_err() {
        log::warn!("Player identity [{}] is not online", ctx.sender);
        return;
    }
    else {
        let player_record = offline_player.unwrap();
        dsl.create_online_player(player_record.get_id(), player_record.identity).expect("Failed to create online player");
        dsl.delete_offline_player_by_id(player_record.get_id());

        log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged in", player_record.id, player_record.identity)
                );
    }
}
