use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};
use std::time::Duration;


#[table(name = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    #[unique]
    pub username: String,
    pub online: bool,
}

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
        // If this is a returning user, i.e. we already have a `User` with this `Identity`,
        // set `online: true`, but leave `name` and `identity` unchanged.
        ctx.db.user().identity().update(User { online: true, ..user });
    } else {
        // If this is a new user, create a `User` row for the `Identity`,
        // which is online, but hasn't set a name.
                // Additional logic, e.g., sending a notification
        log::info!("New User created, set initial username to {}", ctx.sender);
        ctx.db.user().insert(User {
            username: ctx.sender.to_string(),
            identity: ctx.sender,
            online: true,
        });
        update_position(ctx,0.0,0.0,0.0);
    }
}

#[reducer(client_disconnected)]
// Called when a client disconnects from SpacetimeDB database server
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User { online: false, ..user });
    } else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to disconnect without connecting first.
        log::warn!("Disconnect event for unknown user with identity {:?}", ctx.sender);
    }
}

/// Takes a name and checks if it's acceptable as a user's name.
fn validate_name(username: String) -> Result<String, String> {
    if username.is_empty() {
        Err("Names must not be empty".to_string())
    } else {
        Ok(username)
    }
}


#[reducer]
/// Clients invoke this reducer to set their user names.
fn set_user_name(ctx: &ReducerContext, username: String) -> Result<(), String> {
    let username = username.trim().to_string();
    let username = validate_name(username)?;
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        log::info!("User {:?} requested update username to: {}", ctx.sender, username);
        ctx.db.user().identity().update(User { username, ..user });
        
        Ok(())
    } else {
        log::info!("User does not yet exist");
        Err("Cannot set name for unknown user".to_string())
    }
}



#[reducer]
// Called when a client updates their position in the SpacetimeDB
pub fn update_position(ctx: &ReducerContext, x: f64, y: f64, z: f64) {
    log::info!("PositionUpdateCalled");
    if let Some(user) = ctx.db.position().identity().find(ctx.sender) {
        log::info!(
            "User {:?} updated position to: ({}, {}, {})",
            ctx.sender, x, y, z
        );
        ctx.db.position().identity().update(Position { x, y, z, identity: ctx.sender });
    } else {
        // Insert a new position for the user
        ctx.db.position().insert(Position {
            identity: ctx.sender,
            x,
            y,
            z,
        });
    }
}
