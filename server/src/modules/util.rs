use spacetimedb::{reducer, table, Identity, ReducerContext, Table, Timestamp};
use crate::modules::uuid::StUuid;


#[table(name = player_audit, public)]
pub struct PlayerAudit {
    pub player_uuid: StUuid,
    pub login_token: Identity,
    pub action: String,
    pub timestamp: Timestamp,
}

#[reducer]
pub fn log_player_action_audit(ctx: &ReducerContext, uuid: StUuid, action: String) {
    // Get the player record from the database. Created it if need be.
    
    log::debug!("User {:?} performed action: {}", ctx.sender, action);
    ctx.db.player_audit()
        .insert(PlayerAudit {
            player_uuid: uuid,
            login_token: ctx.sender,
            action,
            timestamp: ctx.timestamp,
        });
}
