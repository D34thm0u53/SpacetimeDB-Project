use spacetimedb::{table, Identity, ReducerContext};
use spacetimedsl::{ dsl, Wrapper };

use crate::modules::player::*;



#[dsl(plural_name = owner_identities)]
#[table(name = owner_identity, private)]
pub struct OwnerIdentity {
#[primary_key]
#[use_wrapper(path = crate::modules::player::PlayerAccountId)]
id: u32, // Fk to the player table
pub owner_ident: Identity, // Fk to the player table
}


pub fn try_server_only(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);
    let owner = dsl.get_owner_identity_by_id(PlayerAccountId::new(0)).map_err(|e| format!("Failed to retrieve owner identity: {:?}", e))?;

    if ctx.sender == owner.owner_ident {
        return Ok(());
    } else {
        Err("This reducer can only be called by SpacetimeDB!".to_string())
    }
}

pub fn create_owner_record(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);

    let owner_identity = dsl.get_owner_identity_by_id(PlayerAccountId::new(0));

    if owner_identity.is_ok() {
        return Err("Record already exists".to_string());
    } else {
        let _owner_identity = dsl.create_owner_identity(PlayerAccountId::new(0), ctx.sender)
            .map_err(|e| format!("Failed to create owner identity: {:?}", e))?;
        return Ok(());
    }

}


/// Get players Identity by username. 
/// If the receiver is online, we get their identity from the online player list.
/// 
/// If the receiver is offline, we get their identity from the offline player list.
/// 
/// If the receiver does not exist, we return None.
pub fn get_player_identity_by_username(ctx: &ReducerContext, username: &String) -> Option<Identity> {
    let dsl = dsl(ctx);

    let player_record = dsl.get_player_account_by_username(username)
        .expect("Failed to create entity");

    Some(*player_record.get_identity())
}