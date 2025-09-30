use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::*;
use crate::modules::util::*;
use crate::modules::entity::*;
use crate::modules::roles::*;
use crate::modules::player_status::*;
use crate::common::try_server_or_dev;



use std::time::Duration;




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
    #[referenced_by(path = crate::modules::chat, table = direct_message)]
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

/*
Private Authentication.

Follows Chippy@STDB.discords "So Stupid it works" flow for private authentication without reducer callbacks.

1.  Client Connects.
2.  Added to "Guests" table.
3.  User Provides Auth Key.
4.  If Auth Key is valid: Add to IDENTITIES mutex.
5.  Scheduled reducer processes Authenticated Users.
        Removes From Guests
        Adds to Online Users
            Optionally: removes from Offline Users.

*/
use lazy_static::lazy_static;
use std::sync::Mutex;


lazy_static! {
    static ref IDENTITIES: Mutex<Vec<Identity>> = Mutex::new(Vec::new());
}


/// Table to track guest users who have connected but not yet authenticated.
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

/// Table to store valid authentication keys.
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
        spacetimedb::ScheduleAt::Interval(Duration::from_millis(2000).into()),
        0,
    )?;
    Ok(())
}

#[spacetimedb::reducer]
/// scheduled reducer to process authenticated users from the IDENTITIES queue.
fn process_authenticated_users(ctx: &ReducerContext, _args: AuthProcessSchedule) -> Result<(), String> {
    // Security check - only allow scheduler to call this
    if ctx.sender != ctx.identity() {
        return Err("This reducer can only be called by the scheduler".to_string());
    }
    
    // Get all identities that need processing
    let mut identities = IDENTITIES.lock().unwrap();
    if identities.is_empty() {
        return Ok(()); // Nothing to process
    }


    log::trace!("--> Processing authenticated users");
    log::trace!("Located {} authenticated identities", identities.len());
    
    
    // Process each identity
    let identities_to_process: Vec<Identity> = identities.drain(..).collect();
    
    for identity in identities_to_process {
        match process_authenticated_identity(ctx, identity) {
            Err(e) => {
                log::error!("Failed to process authenticated identity {}: {}", identity, e);
                // Re-add to queue for retry (optional)
                identities.push(identity);
            },
            _ => {}
        }
    }
    

    log::trace!("<-- Completed processing authenticated users");
    Ok(())
}

/// Processes an authenticated identity.
fn process_authenticated_identity(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    //      Remove it from the guest users table.
    //      If a PlayerAccount does not exist, create one with a default username and move to online.
    //      else, move existing player to online.
    //
    //      Arguments:
    //      `ctx` - The reducer context for database operations.
    //      `identity` - The authenticated identity to process.
    //      
    //       Returns:
    //      `Ok(())` on success, or `Err(String)` with an error message on failure.
    let dsl = dsl(ctx);
    
    // Remove from guest users if exists
    if let Ok(_guest_user) = dsl.get_guest_user_by_identity(&identity) {
        match dsl.delete_guest_user_by_identity(&identity) {
            Err(e) => log::warn!("Failed to remove guest user for {}: {:?}", identity, e),
            _ => {}
        }
    }



    // Check if player account exists
    if !does_player_account_exist(ctx, identity) {
        // Create a new player account if it doesn't exist
        let default_username: String = identity.to_string().chars().take(28).collect();
        let player_account = create_player_account(ctx, identity, default_username.clone())?;

        create_related_records_for_playeraccount(ctx, &player_account)?;
    } else {
        log::debug!("Player account already exists for identity [{}]", identity);
        if let Ok(player_account) = dsl.get_player_account_by_identity(&identity) {
            player_account.move_player_to_online(ctx)?;
        }
    }


    

    
    log_player_action_audit(
        ctx,
        &format!("Processed authenticated identity: {}", identity),
    );
    
    Ok(())
}

//// Impls ///
impl PlayerAccount {
    /// Moves the player to offline status.
    fn move_player_to_offline(&self, ctx: &ReducerContext) -> Result<(), String> {
    //      For a given PlayerAccount, move them to offline status.
    //      
    //      If the player is already offline, this is a no-op.
    //      If the player is online, we remove them from online and add to offline.
    //      Players should not be able to reach this point without a valid PlayerAccount.
    //      
    //      Handle Race condition:
    //      If the player disconnects while being processed, ensure they are fully logged out.
    //      This means removing them from online if they are there, and ensuring they are not in
    //      any other state (e.g., guest).
    //       
    //      Revision History:
    //      2025-09-23 - KS - Initial Version

        let dsl = dsl(ctx);

        // Check if already offline
        if dsl.get_offline_player_by_id(&self.get_id()).is_ok() {
            return Ok(()); // Already offline, no-op
        }

        // Remove from online if exists
        if dsl.get_online_player_by_id(&self.get_id()).is_ok() {
            dsl.delete_online_player_by_id(&self.get_id())
                .map_err(|e| format!("Failed to remove from online: {:?}", e))?;
            log::debug!("Removed player [{}] from online", self.get_id());
        } else {
            log::warn!("Player [{}] not found in online when moving to offline, checking for guest", self.get_id());

            // Player is not online, check if they're a guest user
            let guest_user = dsl.get_guest_user_by_identity(&ctx.sender);
            if guest_user.is_ok() {
                // Remove the guest user record
                match dsl.delete_guest_user_by_identity(&ctx.sender) {
                    Ok(_) => {
                        log::debug!("Guest user [{}] disconnected and removed", ctx.sender);
                    },
                    Err(e) => {
                        log::error!("Failed to delete guest user [{}]: {:?}", ctx.sender, e);
                    }
                }
                // RACE CONDITION FIX: Remove identity from authentication queue if present
                // This prevents the scheduled reducer from processing a disconnected player
                {
                    let mut identities = IDENTITIES.lock().unwrap();
                    let original_len = identities.len();
                    identities.retain(|&identity| identity != ctx.sender);
                    let removed_count = original_len - identities.len();
                    
                    if removed_count > 0 {
                        log::warn!("Removed {} pending authentication entries for disconnected player [{}]", removed_count, ctx.sender);
                    }
                }
            } else {
                log::warn!("Player identity [{}] is neither online nor a guest", ctx.sender);
            }
        }
        // Add to offline
        dsl.create_offline_player(self.get_id(), self.identity)
            .map_err(|e| format!("Failed to create offline player: {:?}", e))?;
        log::debug!("Moved player [{}] to offline", self.get_id());
        Ok(())
    }

    /// Moves the player to online status.
    fn move_player_to_online(&self, ctx: &ReducerContext) -> Result<(), String> {
        // For a given PlayerAccount, move them to online status.
        //
        // If the player is already online, this is a no-op.
        // If this is a new player, we create a new OnlinePlayer record.
        // If the player is offline, we remove them from offline and add to online.
        //
        // Revision History:
        // 2025-09-23 - KS - Initial Version

        let dsl = dsl(ctx);
        // Check if already online
        if dsl.get_online_player_by_id(&self.get_id()).is_ok() {
            log::warn!("Player [{}] is already online, should not reach this branch.", self.get_id());
            return Ok(()); // Already online, no-op
        }

        // Remove from offline if exists
        if dsl.get_offline_player_by_id(&self.get_id()).is_ok() {
            dsl.delete_offline_player_by_id(&self.get_id())
                .map_err(|e| format!("Failed to remove from offline: {:?}", e))?;
        }

        // Add to online
        dsl.create_online_player(self.get_id(), self.identity)
            .map_err(|e| format!("Failed to create online player: {:?}", e))?;
        Ok(())
    }
}

/// Validates a username string for use as a player's name.
/// Normalizes the username by trimming whitespace and converting to lowercase.
pub fn normalise_username(username: &String) -> Result<String, String> {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    
    use unicode_normalization::UnicodeNormalization;
    let normalized = username.nfkc().collect::<String>().to_lowercase();
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if trimmed.len() > 32 {
        return Err("Username must be 32 characters or less".to_string());
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("Username contains invalid characters. Only ASCII letters, numbers, underscores (_), and hyphens (-) are allowed.".to_string());
    }
    Ok(trimmed.to_owned())
}
    
/// Handles player connection events such as "connect" and "disconnect".
pub fn handle_player_connection_event(ctx: &ReducerContext, connection_event_type: &str)  {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */

    log::debug!("Handling event [{}] for player: [{}]", connection_event_type, ctx.sender);
    match connection_event_type {
        "connect" => {
            match connect_as_guest(ctx) {
                Ok(guest_user) => {
                    log::debug!("Created new GuestUser: {:?}", guest_user);
                },
                Err(e) => {
                    log::error!("Failed to create guest user: {}", e);
                    return;
                }
            }
            
        },

        "disconnect" => {
            log::debug!("Player [{}] disconnected", ctx.sender);
            // On disconnect, move player to offline if they have a PlayerAccount
            let dsl = dsl(ctx);
            let player_account = dsl.get_player_account_by_identity(&ctx.sender);
            if player_account.is_err() {
                // Player has no account, just remove from guest if exists
                if let Ok(_guest_user) = dsl.get_guest_user_by_identity(&ctx.sender) {
                    match dsl.delete_guest_user_by_identity(&ctx.sender) {
                        Ok(_) => log::debug!("Removed guest user for identity: {}", ctx.sender),
                        Err(e) => log::warn!("Failed to remove guest user for {}: {:?}", ctx.sender, e),
                    }
                } else {
                    log::debug!("No PlayerAccount or GuestUser found for disconnected identity: {}", ctx.sender);
                }
                return;
            }
            let player_account = player_account.unwrap();
            match player_account.move_player_to_offline(ctx) {
                Err(e) => {
                    log::error!("Failed to move player [{}] to offline: {}", player_account.get_id(), e);
                },
                _ => {}
            }
        },
        _ => {
            log::warn!(
                "Unknown player connection event: {} (expected 1=connect, 2=disconnect)",
                connection_event_type
            );
        }
    }
    
}

/// Returns a GuestUser record for the connecting identity, creating one if it doesn't exist.
pub fn connect_as_guest(ctx: &ReducerContext) -> Result<GuestUser, String> {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    let dsl = dsl(ctx);
    // Check if a GuestUser already exists for this identity
    match dsl.get_guest_user_by_identity(&ctx.sender) {
        Ok(existing_guest) => Ok(existing_guest),
        Err(_) => dsl.create_guest_user(ctx.sender)
            .map_err(|e| format!("SpacetimeDSL error: {:?}", e)),
    }
}

/// Generic player account lookup function that returns the full PlayerAccount
pub fn get_player_account(ctx: &ReducerContext, lookup: PlayerAccountLookup) -> Option<PlayerAccount> {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    let dsl = dsl(ctx);
    match lookup {
        PlayerAccountLookup::Id(id) => dsl.get_player_account_by_id(&id).ok(),
        PlayerAccountLookup::Identity(identity) => dsl.get_player_account_by_identity(&identity).ok(),
        PlayerAccountLookup::Username(username) => dsl.get_player_account_by_username(&username).ok(),
    }
}

/// Enum to specify which field to search by
pub enum PlayerAccountLookup {
    Id(PlayerAccountId),
    Identity(Identity),
    Username(String),
}

/// Convenience function to get username by PlayerAccountId
pub fn get_username_by_id(ctx: &ReducerContext, id: PlayerAccountId) -> String {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    get_player_account(ctx, PlayerAccountLookup::Id(id))
        .map(|account| account.get_username().to_string())
        .unwrap_or_default()
}

/// Convenience function to get username by identity    
pub fn get_username_by_identity(ctx: &ReducerContext, identity: Identity) -> String {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    get_player_account(ctx, PlayerAccountLookup::Identity(identity))
        .map(|account| account.get_username().to_string())
        .unwrap_or_default()
}

/// Convenience function to get Identity by username
pub fn get_identity_by_username(ctx: &ReducerContext, username: String) -> Option<Identity> {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    get_player_account(ctx, PlayerAccountLookup::Username(username))
        .map(|account| account.identity)
}

/// Checks if a PlayerAccount exists for the given identity.
pub fn does_player_account_exist(ctx: &ReducerContext, identity: Identity) -> bool {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    */
    let dsl = dsl(ctx);
    dsl.get_player_account_by_identity(&identity).is_ok()
}

/// Create a new PlayerAccount record for a given Identity and username.
fn create_player_account(ctx: &ReducerContext, identity: Identity, username: String) -> Result<PlayerAccount, String> {
    /* Revision History:
    2025-09-23 - KS - Initial Version
    2025-09-30 - KS - Moved 'related' logic to create_related_records_for_playeraccount

    */
    
    let dsl = dsl(ctx);

    match normalise_username(&username) {
        Ok(validated_username) => {
            if validated_username != username {
                log::warn!("Username [{}] was normalized to [{}]", username, validated_username);
            }
        },
        Err(e) => {
            return Err(format!("Invalid username: {}", e));
        }
    }

    // Check if username is already taken
    if dsl.get_player_account_by_username(&username).is_ok() {
        return Err("Username already taken".to_string());
    }
    // Create PlayerAccount
    let player_account = dsl.create_player_account(identity.clone(), &username)
        .map_err(|e| format!("Failed to create PlayerAccount: {:?}", e))?;


    
    
    Ok(player_account)
}

/// For a given PlayerAccount, related records in linked tables.
fn create_related_records_for_playeraccount(ctx: &ReducerContext, player_account: &PlayerAccount) -> Result<(), String> {
    /*  Revision History:
        2025-09-28 - KS - Initial Version
        2025-09-28 - KS - Added Role creation
        2025-09-28 - KS - Added PlayerStatus creation
        2025-09-28 - KS - Added Entity creation
        2025-09-30 - KS - Added username normalization
    */

    let dsl = dsl(ctx);
    // Security check - only allow server or developer to call this
    if !try_server_or_dev(ctx) {
        return Err("Unauthorized access".to_string());
    }

    // Create default role profile
    create_default_roles(&dsl, player_account.get_id())?;

    // Move the player to online - Creating the OnlinePlayer record if it doesn't exist yet
    player_account.move_player_to_online(ctx)?;
    
    // Create default PlayerStatus record.
    PlayerStatus::create_default_state(&dsl, player_account.get_id());

    // Create entity records:
    //  entity_rotation
    //  entity_position
    //  entity_chunk
    create_entity_tree(ctx, EntityType::Player);

    Ok(())

}

/// Reducers

#[spacetimedb::reducer]
// Sets the username for the requesting player, ensuring uniqueness and validity.
pub fn set_username(ctx: &ReducerContext, t_username: String) -> Result<(), String> {
     let dsl = dsl(ctx);

    // Get a normalised version of the requested name.
    let normalised_username = normalise_username(&t_username)?;


    // Check if the username already exists in the database
    let player_account_with_requested_username = dsl.get_player_account_by_username(&normalised_username);
    
    match player_account_with_requested_username {
        Err(_) => {
            // Username does not exist, proceed
        },
        Ok(existing_account) => {
            // Username exists, check if it's the requesting user
            if existing_account.identity == ctx.sender {
                // The requesting user already has this username set, no-op
                return Ok(());
            }
            return Err("Username already taken".to_string());
        }
    }
    
    let mut requesting_user_account = dsl.get_player_account_by_identity(&ctx.sender)?;
    requesting_user_account.username = normalised_username.clone();
    

    log_player_action_audit(
        ctx,
        &format!(
            "Player [{}] (Identity: [{}]) set username to [{}]",
            &requesting_user_account.get_id(),
            &requesting_user_account.get_identity(),
            &normalised_username
        ),
    );

    dsl.update_player_account_by_identity(requesting_user_account)?;

    Ok(())
}
   
