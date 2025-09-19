use spacetimedb::{table, Identity, ReducerContext, Timestamp};
use spacetimedsl::{dsl};

#[dsl(plural_name = player_audits)]
#[table(name = player_audit, public)]
pub struct PlayerAudit {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
    pub user_identity: Identity,
    pub action: String,
    created_at: Timestamp,
}

pub fn log_player_action_audit(ctx: &ReducerContext, action: &str) {
    let dsl = dsl(ctx);
    dsl
        .create_player_audit(ctx.sender, action)
        .expect("Failed to create audit record");
}

