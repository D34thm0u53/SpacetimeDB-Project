use spacetimedb::{reducer, spacetimedb_lib::identity, table, Identity, ReducerContext, Table, Timestamp};

// Store current location of users
#[table(name = player, public)]
pub struct Player {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    identity: Identity,
    pub online: bool,
    pub last_seen: Timestamp,
    #[unique]
    pub username: String,
}

#[reducer]
pub fn player_login(ctx: &ReducerContext ) {
    // Check if the player already exists in the database
    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Player already exists, update their online status
        ctx.db.player().id().update(Player { online: true, last_seen: ctx.timestamp, ..player });
    } else {
        // This is a new player, create a new entry in the database
        ctx.db.player().insert(Player {
            id: 0,
            identity: ctx.sender,
            online: true,
            last_seen: ctx.timestamp,
            username: "".to_string(), // Default username
        });
    }
}

#[reducer]
pub fn player_logout(ctx: &ReducerContext ) {
    // Check if the player already exists in the database
    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Player already exists, update their online status
        ctx.db.player().id().update(Player { online: false, last_seen: ctx.timestamp, ..player });
    } else {
        // This should not be reachable,
        // as it doesn't make sense for a player to log out without logging in first.
        log::warn!("Player {} logged out without logging in first", ctx.sender);
        
        // This is a new player, create a new entry in the database
        ctx.db.player().insert(Player {
            id: 0,
            identity : ctx.sender,
            online: true,
            last_seen: ctx.timestamp,
            username: "".to_string(), // Default username
        });
    }
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
    if username.len() > 32 {
        Err("Names must be less than 32 characters".to_string())
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
    else if username.is_empty() {
        Err("Names must not be empty".to_string())
    }
    else {
        Ok(username)
    }
}