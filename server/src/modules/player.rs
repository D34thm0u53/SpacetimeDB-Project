use spacetimedb::{table, Identity, ReducerContext, Timestamp};

use spacetimedsl::*;
use spacetimedsl::hook;

use crate::modules::util::*;
use crate::modules::entity::entity::*;
use crate::modules::roles::*;
use crate::modules::player_status::*;


use spacetimedb::{view, ViewContext};


#[view(name = my_player, public)]
fn my_player(ctx: &ViewContext) -> Option<PlayerAccount> {
    ctx.db.player_account().identity().find(ctx.sender)
}



#[dsl(plural_name = player_accounts,
    method(
        update = true,
        delete = true
    ),
    hook(
        after(insert)
    )
)]
#[table(name = player_account, public)]
pub struct PlayerAccount {
    #[primary_key]
    #[index(btree)]
    #[auto_inc]
    #[create_wrapper]
    #[referenced_by(path = crate, table = online_player)]
    #[referenced_by(path = crate, table = offline_player)]
    #[referenced_by(path = crate::modules::roles, table = role)]
    #[referenced_by(path = crate::modules::entity::entity, table = entity)]
    #[referenced_by(path = crate::modules::chat, table = direct_message)]
    pub id: u32, // Auto-incremented ID for the player record
    #[unique]
    pub identity: Identity,
    #[unique]
    pub username: String,
    created_at: Timestamp,
    modified_at: Timestamp,
}

// online_player is a table that stores currently online players.
#[dsl(plural_name = online_players,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = online_player, public)]
pub struct OnlinePlayer {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(PlayerAccountId)]
    #[foreign_key(path = crate, column = id, table = player_account, on_delete = Error)]
    id: u32,
    #[unique]
    pub identity: Identity,
    created_at: Timestamp,
}

// offline_player is a table that stores currently offline players.
#[dsl(plural_name = offline_players,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = offline_player, public)]
pub struct OfflinePlayer {
    #[primary_key]
    #[index(btree)]
    #[use_wrapper(PlayerAccountId)]
    #[foreign_key(path = crate, column = id, table = player_account, on_delete = Delete)]
    id: u32,
    #[unique]
    pub identity: Identity,
    created_at: Timestamp,
}

//// Impls ///

/// Creates related records (role, online status, player status, entity tree) for a new player account.
#[hook]
fn after_player_account_insert(dsl: &spacetimedsl::DSL, row: &PlayerAccount) -> Result<(), SpacetimeDSLError> {
    create_default_roles(&dsl, row.get_id())?;

    // Move the player to online - Creating the OnlinePlayer record if it doesn't exist yet
    row.move_player_to_online(dsl.ctx())?;

    // Create default PlayerStatus record.
    PlayerStatus::create_default_state(&dsl, row.get_id());

    // Create entity records:
    //  entity_rotation
    //  entity_position
    //  entity_chunk
    create_entity_tree(dsl.ctx(), EntityType::Player);

    Ok(())

}


impl PlayerAccount {
    /// Transitions player to offline status, removing from online table and cleaning up schedulers if no players remain.
    fn move_player_to_offline(&self, ctx: &ReducerContext) -> Result<(), String> {
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
            log::warn!("Player [{}] not found in online when moving to offline", self.get_id());
        }
        // Add to offline
        dsl.create_offline_player(CreateOfflinePlayer {
            id: self.get_id(),
            identity: self.identity,
        })
            .map_err(|e| format!("Failed to create offline player: {:?}", e))?;
        log::debug!("Moved player [{}] to offline", self.get_id());
        

        // DEV
        /*
        While we are in no/low player count, we should wind down resources if there are no other users connected.

        Remove scheduled reducers:
            Chunk Calculation
            Message Archive
        */
        
        use crate::schedulers::scheduler_chunks::*;
        use crate::schedulers::scheduler_chat_archive::*;

        let playercount = dsl.count_of_all_online_players();
        log::debug!("Current online player count: {}", playercount);

        if playercount > 0 {
            return Ok(());
        }

        let chat_archive_timers = dsl.get_all_chat_archive_timers();
        for timer in chat_archive_timers {
            log::debug!("Deleting chat archive timer ID: {}", timer.get_id());
            dsl.delete_chat_archive_timer_by_id(&timer.get_id())?;
        }
                
        let chunk_timers = dsl.get_all_chunk_check_timers();
        for timer in chunk_timers {
            log::debug!("Deleting chunk check timer ID: {}", timer.get_id());
            dsl.delete_chunk_check_timer_by_id(&timer.get_id())?;
        }
        Ok(())
    }

    /// Transitions player to online status, removing from offline table and starting schedulers.
    fn move_player_to_online(&self, ctx: &ReducerContext) -> Result<(), SpacetimeDSLError> {
        use crate::schedulers::scheduler_chunks::wrap_create_chunk_check_timer;
        use crate::schedulers::scheduler_chat_archive::wrap_create_chat_archive_timer;

        let dsl = dsl(ctx);
        // Check if already online
        if dsl.get_online_player_by_id(&self.get_id()).is_ok() {
            log::warn!("Player [{}] is already online, should not reach this branch.", self.get_id());
            return Ok(()); // Already online, no-op
        }

        // Remove from offline if exists
        if dsl.get_offline_player_by_id(&self.get_id()).is_ok() {
            dsl.delete_offline_player_by_id(&self.get_id())?;
        }

        // Add to online
        dsl.create_online_player(CreateOnlinePlayer {
            id: self.get_id(),
            identity: self.identity,
        })?;

        // Start a scheduled reducer if not running
        let _ = wrap_create_chunk_check_timer(ctx);        
        let _ = wrap_create_chat_archive_timer(ctx);
        Ok(())

    }
}

/// Normalizes username (NFKC, lowercase, trim) and validates length constraints.
pub fn normalise_username(username: &String) -> Result<String, String> {
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
    2025-11-09 - KS - Removed guest user authentication flow
    */

    log::debug!("Handling event [{}] for player: [{}]", connection_event_type, ctx.sender);
    match connection_event_type {
        "connect" => {
            log::debug!("Player [{}] connected", ctx.sender);
            // Authentication is now handled by SpaceTimeAuth

            // Here we will validate the issuer identity and create a PlayerAccount if it doesn't exist.
            
            // for now, just create if not exists
            
            match does_player_account_exist(ctx, ctx.sender) {

                true => {
                    log::debug!("PlayerAccount already exists for identity: {}", ctx.sender);
                },
                false => {
                    // Create a default username based on identity
                    let default_username = format!("player_{}", ctx.sender.to_string().chars().take(16).collect::<String>());
                    match create_player_account(ctx, ctx.sender, default_username) {
                        Ok(account) => {
                            log::info!("Created PlayerAccount [{}] for new identity: {}", account.get_id(), ctx.sender);
                        },
                        Err(e) => {
                            log::error!("Failed to create PlayerAccount for identity [{}]: {}", ctx.sender, e);
                        }
                    }
                }
            }


        },

        "disconnect" => {
            log::debug!("Player [{}] disconnected", ctx.sender);
            // On disconnect, move player to offline if they have a PlayerAccount
            let dsl = dsl(ctx);
            let player_account = dsl.get_player_account_by_identity(&ctx.sender);
            let player_account = match player_account {
                Ok(account) => account,
                Err(_) => {
                    log::warn!("No PlayerAccount found for disconnected identity: {}", ctx.sender);
                    return;
                }
            };
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

/// Retrieves a PlayerAccount by ID, identity, or username.
pub fn get_player_account(ctx: &ReducerContext, lookup: PlayerAccountLookup) -> Option<PlayerAccount> {
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

/// Retrieves the username for a player by their PlayerAccountId.
pub fn get_username_by_id(ctx: &ReducerContext, id: PlayerAccountId) -> String {
    get_player_account(ctx, PlayerAccountLookup::Id(id))
        .map(|account| account.get_username().to_string())
        .unwrap_or_default()
}

/// Retrieves the username for a player by their Identity.
pub fn get_username_by_identity(ctx: &ReducerContext, identity: Identity) -> String {
    get_player_account(ctx, PlayerAccountLookup::Identity(identity))
        .map(|account| account.get_username().to_string())
        .unwrap_or_default()
}

/// Retrieves the Identity for a player by their username.
pub fn get_identity_by_username(ctx: &ReducerContext, username: String) -> Option<Identity> {
    get_player_account(ctx, PlayerAccountLookup::Username(username))
        .map(|account| account.identity)
}

/// Checks if a PlayerAccount exists for the given identity.
pub fn does_player_account_exist(ctx: &ReducerContext, identity: Identity) -> bool {
    let dsl = dsl(ctx);
    dsl.get_player_account_by_identity(&identity).is_ok()
}

/// Creates a new PlayerAccount with normalized username.
fn create_player_account(ctx: &ReducerContext, identity: Identity, username: String) -> Result<PlayerAccount, String> {
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
    let player_account = dsl.create_player_account(CreatePlayerAccount {
        identity,
        username: username.clone(),
    })
        .map_err(|e| format!("Failed to create PlayerAccount: {:?}", e))?;


    
    
    Ok(player_account)
}


/// Reducers

/// Updates the username for the requesting player after validation and uniqueness check.
#[spacetimedb::reducer]
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
   
