use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};

/*
Define our Tables

*/

// Intentionally private
#[spacetimedb::table(name = update_config)]
pub struct UpdateConfig {
    #[unique]
    id: u32,
    value: i32,
}


// Store User Profiles
#[table(name = user, public)]
pub struct User {
    #[primary_key]
    identity: Identity,
    username: String,
    online: bool,
    last_seen: Timestamp,
}

// Store User Roles
#[table(name = roles, private)]
pub struct Roles {
    #[primary_key]
    identity: Identity,
    is_administrator: bool, 
    is_moderator: bool,
}

// Store current location of users
#[table(name = chunk, public)]
pub struct Chunk {
    #[primary_key]
    identity: Identity,
    x: u32,
    y: u32,
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


// Structure for the internal entity position table
// This table is used to store the position of entities in the game world.
// It is not intended to be accessed directly by clients, hence the private access modifier.

// Intentionally private
#[spacetimedb::table(name = internal_entity_position, private)]
pub struct InternalEntityPosition {
    #[unique]
    pub id: u32,
    pub transform: StdbTransform,
}



// Structure for the entity Transform table
#[spacetimedb::table(name = stdb_transform, private)]
#[derive(Clone)]
pub struct StdbTransform {
    position: StdbPosition,
    rotation: StdbRotation,
}

// Structure for the entity position table
#[spacetimedb::table(name = stdb_position, private)]
#[derive(Clone)]
pub struct StdbPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
// Structure for the entity rotation table
#[spacetimedb::table(name = stdb_rotation, private)]
#[derive(Clone)]
pub struct StdbRotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}



// Structure for the non-player entity table
#[spacetimedb::table(name = entity, public)]
pub struct Entity {
    #[unique]
    id: u32,
    #[unique]
    identity: Identity,
    #[unique]
    username: String,
}

// Structure for the player entity table
#[table(name = player_entity, public)]
pub struct PlayerEntity {
    #[primary_key]
    identity: Identity,
    transform: StdbTransform,
}


// Structure for the table containing the scheduled update position timer
#[spacetimedb::table(name = update_position_timer, scheduled(update_all_positions))]
pub struct UpdatePositionTimer {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}

// We'll update this table 20 times per second
#[spacetimedb::table(name = entity_position_hr, public)]
pub struct EntityPositionHR {
    #[unique]
    pub id: u32,
    pub transform: StdbTransform,
}
    
// We'll only update this table 5 times per second
#[spacetimedb::table(name = entity_position_lr, public)]
pub struct EntityPositionLR {
    #[unique]
    pub id: u32,
    // You could make other types here which use f16 or another
    // representation to save even more space
    pub transform: StdbTransform,
}


#[spacetimedb::reducer]
pub fn update_my_position(ctx: &ReducerContext, transform: StdbTransform) {
    // The user has provided us with an update of their current position
    
    if let Some(player_entity) = ctx.db.player_entity().identity().find(ctx.sender) {
        // Update the user's internal position
        ctx.db.player_entity().identity().update(PlayerEntity { 
            transform,
            ..player_entity
        });

    } else {
        // This is a new player, so we need to create one.
        log::trace!("New Player created, set initial username to {}", ctx.sender);

        ctx.db.player_entity().insert(PlayerEntity {
            identity: ctx.sender,
            transform,
        });
    }
}



#[spacetimedb::reducer]
pub fn update_position(ctx: &ReducerContext, transform: StdbTransform) {
    // We'll update this user's internal position, not their public position
    let entity = ctx.db.entity().identity().find(ctx.sender).unwrap();
    if ctx
        .db
        .internal_entity_position()
        .id()
        .find(entity.id)
        .is_some()
    {
        ctx.db
            .internal_entity_position()
            .id()
            .update(InternalEntityPosition {
                id: entity.id,
                transform,
                
            });
    } else {
        ctx.db
            .internal_entity_position()
            .insert(InternalEntityPosition {
                id: entity.id,
                transform,
            });
    }
}

#[spacetimedb::reducer]
pub fn update_all_positions(ctx: &ReducerContext, _arg: UpdatePositionTimer) {
    // We're using this value to determine whether or not to update the lower resolution table.
    // Here we're doing a 4:1 ratio (4 high resolution updates for every 1 low resolution update)
    let mut update = ctx.db.update_config().id().find(0).unwrap();
    // Only let SpacetimeDB call this function
    if ctx.sender != ctx.identity() {
        panic!("wrong owner! This reducer can only be called by SpacetimeDB!");
    }

    let low_resolution = update.value == 0;
    // Update the value in the config table
    update.value = (update.value + 1) % 4;
    ctx.db.update_config().id().update(update);


    // Clear all high res positions
    for row in ctx.db.entity_position_hr().iter() {
        ctx.db.entity_position_hr().id().delete(row.id);
    }

    if low_resolution {
        // Clear all low res positions
        for row in ctx.db.entity_position_lr().iter() {
            ctx.db.entity_position_lr().id().delete(row.id);
        }
    }

    // Update all high res positions
    for row in ctx.db.internal_entity_position().iter() {
        ctx.db.entity_position_hr().insert(EntityPositionHR {
            id: row.id,
            transform: row.transform.clone(),
        });

        if low_resolution {
            ctx.db.entity_position_hr().insert(EntityPositionHR {
                id: row.id,
                transform: row.transform,
            });
        }
    }
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
        log::trace!("New User created, set initial username to {}", ctx.sender);
        ctx.db.user().insert(User {
            username: ctx.sender.to_string(),
            identity: ctx.sender,
            online: true,
            last_seen: ctx.timestamp,
        });
        //for all new users, also create a row in the position table
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
        log::debug!("User {:?} requested update username to: {}", ctx.sender, username);
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
fn _set_user_name_override(ctx: &ReducerContext, username: String, user_identity: Identity) -> Result<(), String> {
    if let Some(roles) = ctx.db.roles().identity().find(ctx.sender) {
        if !roles.is_moderator && !roles.is_administrator {
            return Err("Only moderators can set names for other users".to_string());
        } else {
        }
    }

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




pub fn is_crossing_chunk(ctx: &ReducerContext, x: f64, y: f64, z: f64) -> bool {
    if let Some(position) = ctx.db.position().identity().find(ctx.sender) {
        let dx = position.x - x;
        let dy = position.y - y;
        let dz = position.z - z;

        if dx.abs() > 1.0 || dy.abs() > 1.0 || dz.abs() > 1.0 {
            return true;
        }
    }
    false
}

pub fn get_position(ctx: &ReducerContext) -> Option<(f64, f64, f64)> {
    if let Some(position) = ctx.db.position().identity().find(ctx.sender) {
        Some((position.x, position.y, position.z))
    }
    else {
        None
    }
}























#[reducer]
// Called when a client updates their position in the SpacetimeDB
pub fn set_position(ctx: &ReducerContext, x: f64, y: f64, z: f64) {
    if let Some(_identity) = ctx.db.position().identity().find(ctx.sender) {
        log::trace!(
            "User {:?} used position override to: ({}, {}, {})",
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

