use spacetimedb::{reducer, spacetimedb_lib::identity, table, Identity, ReducerContext, Table, Timestamp};

// Store current location of users
#[table(name = player, public)]
pub struct Player {
    #[primary_key]
    #[auto_inc]
    id: u64,
    #[unique]
    identity: Identity,
    online: bool,
    last_seen: Timestamp,
    
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
        });
    }
}
