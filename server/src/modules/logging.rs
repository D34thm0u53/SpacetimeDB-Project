use spacetimedb::{table, Identity, ReducerContext, Timestamp, SpacetimeType};
use spacetimedsl::dsl;





#[dsl(plural_name = event_logs)]
#[table(name = event_log, public)]
pub struct EventLog {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub user: Identity,
    pub description: String,
    
}

#[dsl(plural_name = audit_logs)]
#[table(name = audit_log, public)]
pub struct AuditLog {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub user: Identity,
    pub description: String,
    created_at: Timestamp,
    modified_at: Timestamp,
}


pub fn log_event(ctx: &ReducerContext, description: String) {
    let dsl = dsl(ctx);
    
    dsl.create_event_log(ctx.sender, &description);
}


pub fn log_audit(ctx: &ReducerContext, description: String) {
    let dsl = dsl(ctx);

    dsl.create_audit_log(ctx.sender, &description);

}
