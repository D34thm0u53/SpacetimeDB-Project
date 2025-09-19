use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::*;
use crate::modules::util::*;
use crate::modules::entity::*;

use crate::modules::player_status::*;
use std::time::Duration;


/*
Private Authentication.

Follows Chippy@STDB.discords "So Stupid it works" flow for private authentication without reducer callbacks.

Client Connects.
Added to "Guests" table.
User Provides Auth Key.
If Auth Key is valid: Add to IDENTITIES mutex.
Scheduled reducer processes Authenticated Users.
    Removes From Guests
    Adds to Online Users
        Optionally: removes from Offline Users.

*/

use lazy_static::lazy_static;
use std::sync::Mutex;
use spacetimedb::{ScheduleAt, TimeDuration};


lazy_static! {
    static ref IDENTITIES: Mutex<Vec<Identity>> = Mutex::new(Vec::new());
}

#[dsl(plural_name = guest_users)]
#[table(name = guest_user, private)]
pub struct GuestUser {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32, // Auto-incremented ID for the key record
    #[unique]
    pub identity: Identity,
}

#[dsl(plural_name = auth_keys)]
#[table(name = auth_key, private)]
pub struct AuthKey {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32, // Auto-incremented ID for the key record
    #[unique]
    pub key_name: String,
    #[unique]
    pub key: String,
}


#[spacetimedb::reducer]
pub fn private_authenticate(ctx: &ReducerContext, key: String) {
    let dsl = dsl(ctx);

    if let Ok(auth_key) = dsl.get_auth_key_by_key_name(&"primary_auth") {
        if key == auth_key.key {
            IDENTITIES.lock().unwrap().push(ctx.sender);
        }
    }
}

/* Now we have a scheduled reducer, every 1 second, that checks the length of IDENTITIES.

For each identity in this list, we find the corresponding player account and mark it as online, removing it from the list of guest users.

*/ 


// Schedule table for the authentication processor
#[dsl(plural_name = auth_process_schedules)]
#[table(name = auth_process_schedule, scheduled(process_authenticated_users), private)]
pub struct AuthProcessSchedule {
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
    dsl.create_auth_process_schedule(
        spacetimedb::ScheduleAt::Interval(Duration::from_secs(60).into()),
        0,
    )?;
    Ok(())
}



#[spacetimedb::reducer]
pub fn process_authenticated_users(ctx: &ReducerContext, _args: AuthProcessSchedule) -> Result<(), String> {
    // Security check - only allow scheduler to call this
    if ctx.sender != ctx.identity() {
        return Err("This reducer can only be called by the scheduler".to_string());
    }

    let dsl = dsl(ctx);
    
    // Get all identities that need processing
    let mut identities = IDENTITIES.lock().unwrap();
    if identities.is_empty() {
        return Ok(()); // Nothing to process
    }

    log::info!("Processing {} authenticated identities", identities.len());
    
    // Process each identity
    let identities_to_process: Vec<Identity> = identities.drain(..).collect();
    
    for identity in identities_to_process {
        match process_authenticated_identity(ctx, identity) {
            Ok(_) => {
                log::info!("Successfully processed authenticated identity: {}", identity);
            },
            Err(e) => {
                log::error!("Failed to process authenticated identity {}: {}", identity, e);
                // Re-add to queue for retry (optional)
                identities.push(identity);
            }
        }
    }
    
    Ok(())
}

fn process_authenticated_identity(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    let dsl = dsl(ctx);
    
    // Remove from guest users if exists
    if let Ok(_guest_user) = dsl.get_guest_user_by_identity(&identity) {
        match dsl.delete_guest_user_by_identity(&identity) {
            Ok(_) => log::info!("Removed guest user for identity: {}", identity),
            Err(e) => log::warn!("Failed to remove guest user for {}: {:?}", identity, e),
        }
    }
    
    // Check if player account exists
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
                    }
                }
            } else {
                log::info!("Player account already exists for identity [{}]", ctx.sender);
                move_player_to_online(ctx)
            }
            log::info!("Player [{}] moved to online.", ctx.sender);
    
    log_player_action_audit(
        ctx,
        &format!("Processed authenticated identity: {}", identity),
    );
    
    Ok(())
}










#[dsl(plural_name = player_accounts)]
#[table(name = player_account, public)]
pub struct PlayerAccount {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    #[referenced_by(path = crate, table = online_player)]
    #[referenced_by(path = crate, table = offline_player)]
    #[referenced_by(path = crate::modules::roles, table = role)]
    #[referenced_by(path = crate::modules::entity, table = entity)]
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
pub fn set_username(ctx: &ReducerContext, t_username: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    log::trace!("Attempting to set username for player: [{}] to [{}]", ctx.sender, t_username);

    let b_is_requested_username_valid = validate_name(t_username.clone());

    if b_is_requested_username_valid.is_err() {
        log::debug!("Invalid username: {}", b_is_requested_username_valid.unwrap_err());
        return Err("Invalid username requested".to_string());
    }

    let t_requested_username = b_is_requested_username_valid.unwrap();

    // Check if the username already exists in the database
    
    let player_account_with_requested_username = dsl.get_player_account_by_username(&t_requested_username);
    // Check if the username belongs to the requesting player
    if player_account_with_requested_username.is_ok() {
        if player_account_with_requested_username.unwrap().identity == ctx.sender {
            // The requesting user already has this username set, no-op
            return Ok(());
        }
        log::debug!("Username [{}] already exists", t_requested_username);
        return Err("Username already taken".to_string());
    }

    // If the username is not taken, we can proceed with the update
    // Get the player_account record of the requesting user.

    let current_user_player_account = dsl.get_player_account_by_identity(&ctx.sender);

    if current_user_player_account.is_err() {
        log::debug!("Failed to get current user player account");
        return Err("Failed to get current user player account".to_string());
    }

    let mut current_user_player_account = current_user_player_account.unwrap();


    // Update username using generated setter
    current_user_player_account.set_username(&t_requested_username);

    log_player_action_audit(
        ctx,
        &format!(
            "Player [{}] (Identity: [{}]) set username to [{}]",
            current_user_player_account.get_id(),
            current_user_player_account.get_identity(),
            t_requested_username
        ),
    );

    // Persist the change to the PlayerAccount
    dsl.update_player_account_by_identity(current_user_player_account).expect("Failed to update player record");
    Ok(())
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

    // Created records in related tables
    create_entity_tree(ctx, EntityType::Player);
    dsl.create_player_status(player_account.get_id(), identity, 500, 1000, 0.0, 0.0, 0.0, 0.0).map_err(|e| format!("Failed to create PlayerStatus: {:?}", e))?;

    
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
        match dsl.delete_online_player_by_id(player_record.get_id()) {
            Ok(_) => {},
            Err(e) => {
                log::error!("Failed to delete online player: {:?}", e);
            }
        }

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
        match dsl.delete_offline_player_by_id(player_record.get_id()) {
            Ok(_) => {},
            Err(e) => {
                log::error!("Failed to delete offline player: {:?}", e);
            }
        }

        log_player_action_audit(
                ctx,
                &format!("Player [{}] (Identity: [{}]) logged in", player_record.get_id(), player_record.identity)
                );
    }
}
