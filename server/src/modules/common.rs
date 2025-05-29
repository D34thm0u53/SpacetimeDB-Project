use spacetimedb::{table, Identity, ReducerContext, Timestamp, SpacetimeType};
use spacetimedsl::dsl;




#[dsl(plural_name = OwnerIdentities)]
#[table(name = owner_identity, private)]
pub struct OwnerIdentity {
#[primary_key]
pub id: u16, // Fk to the player table
pub owner_identity: Identity, // Fk to the player table
}

pub fn try_server_only(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);
    let Some(owner) = dsl.get_owner_identity_by_id(&0) else {
        return Err("Owner identity not found".to_string());
    };

    if ctx.sender == owner.owner_identity {
        return Ok(());
    }
    else {
        Err("This reducer can only be called by SpacetimeDB!".to_string())
    }   
}

pub fn create_owner_record(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);
    if dsl.get_owner_identity_by_id(&0).is_some() {
        return Err("Record already exists".to_string());
    }
    else {
        dsl.create_owner_identity(0, ctx.sender).expect("Failed to create owner identity");
        return Ok(()); // Owner record already exists
    }

}
