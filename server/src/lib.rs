use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};

/*
Define our Tables

*/

// Store User Profiles
#[table(name = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    pub username: String,
    pub online: bool,
    pub last_seen: Timestamp,
}

// Store current location of users
#[table(name = position, public)]
pub struct Position {
    #[primary_key]
    identity: Identity,
    x: f64,
    y: f64,
    z: f64,
}



#[reducer(client_connected)]
// Called when a client connects to a SpacetimeDB database server
pub fn client_connected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        // Update the user's online status to true
        ctx.db.user().identity().update(User { online: true, ..user });
    } 
    else {
        //this is a new user, so we need to create one.
        log::info!("New User created, set initial username to {}", ctx.sender);
        ctx.db.user().insert(User {
            username: ctx.sender.to_string(),
            identity: ctx.sender,
            online: true,
            last_seen: ctx.timestamp,
        });
        //for all new users, also create a row in the position table
        update_position(ctx,0.0,0.0,0.0);
    }
}


#[reducer(client_disconnected)]
// Called when a client disconnects from SpacetimeDB database server
pub fn client_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User { online: false, last_seen: ctx.timestamp, ..user });
    }
    else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to disconnect without connecting first.
        log::warn!("Disconnect event for unknown user with identity {:?}", ctx.sender);
    }
}


// Name Management
#[reducer]
/// Clients invoke this reducer to set their user names.
fn set_user_name(ctx: &ReducerContext, username: String) -> Result<(), String> {
    let username = username.trim().to_string();
    let username = validate_name(username)?;
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        log::info!("User {:?} requested update username to: {}", ctx.sender, username);
        ctx.db.user().identity().update(User { username, ..user });
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

// Moderator Name Management
fn set_user_name_override(ctx: &ReducerContext, username: String, user_identity: Identity) -> Result<(), String> {
    let username = username.trim().to_string(); // Even for moderators, we need to ensure there is no whitespace in the name.
    // They however get away wioth a few more characters and can try break stuff
    if let Some(user) = ctx.db.user().identity().find(user_identity) {
        log::info!("Moderator User {:?} Applied username update to target: {}. Name set to: {}", ctx.sender,user_identity, username);
        ctx.db.user().identity().update(User { username, ..user });
        Ok(())
    }
    else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to set username without connecting first.
        Err("Cannot set name for unknown user".to_string())
    }
}


#[reducer]
// Called when a client updates their position in the SpacetimeDB
pub fn update_position(ctx: &ReducerContext, x: f64, y: f64, z: f64) {
    log::info!("PositionUpdateCalled");
    if let Some(_identity) = ctx.db.position().identity().find(ctx.sender) {
        log::info!(
            "User {:?} updated position to: ({}, {}, {})",
            ctx.sender, x, y, z
        );
        ctx.db.position().identity().update(Position { x, y, z, identity: ctx.sender });
    }
    else {
        // Insert a new position for the user
        ctx.db.position().insert(Position {
            identity: ctx.sender,
            x,
            y,
            z,
        });
    }
}
