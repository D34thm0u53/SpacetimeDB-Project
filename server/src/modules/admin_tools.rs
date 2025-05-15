use spacetimedb::{reducer, spacetimedb_lib::identity, table, Identity, ReducerContext, Table, Timestamp};
use crate::modules::player::{player, Player};
use crate::modules::roles::{Roles, RoleType,roles};


#[reducer]
pub fn validate_users(ctx: &ReducerContext) {
    // Iterate through the player table
    log::debug!("Validating users...");
    for player in ctx.db.player().iter() {
        // Ensure all users have a username
        log::debug!(
            "Validating user: id={}, username={}, identity={}",
            player.id,
            player.username,
            player.identity
        );

        if player.username.is_empty() {
            let truncated_identity = player.identity.to_string().chars().take(32).collect::<String>();
            ctx.db.player().id().update(Player {
                username: truncated_identity,
                ..player
            });
        }

        // Ensure all users have an entry in the roles table
        if ctx.db.roles().id().find(player.id).is_none() {
            ctx.db.roles().insert(Roles {
                id: 0,
                identity: player.identity,
                is_trusted_user: false,
                is_game_admin: false,
                is_server_administrator: false,
            });
        }
    }
log::debug!("User validation complete.");

}