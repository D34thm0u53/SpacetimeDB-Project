use spacetimedb::{reducer, ReducerContext, Table,};
use crate::modules::player::{player, Player};




#[reducer]
pub fn validate_users(ctx: &ReducerContext) {
    // Iterate through the player table
    log::debug!("Validating users...");
    for player in ctx.db.player().iter() {
        // Ensure all users have a username
        log::debug!(
            "Validating user: id={}, username={}, identity={}",
            player.uuid,
            player.username,
            player.identity
        );

        if player.username.is_empty() {
            let truncated_identity = player.identity.to_string().chars().take(32).collect::<String>();
            ctx.db.player().uuid().update(Player {
                username: truncated_identity,
                ..player
            });
        }

    }
log::debug!("User validation complete.");

}

