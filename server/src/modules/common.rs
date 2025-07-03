use spacetimedb::{table, Identity, ReducerContext};
use spacetimedsl::dsl;

use super::player::*;


#[dsl(plural_name = OwnerIdentities)]
#[table(name = owner_identity, private)]
pub struct OwnerIdentity {
#[primary_key]
#[wrap]
id: u16, // Fk to the player table
pub owner_ident: Identity, // Fk to the player table
}

pub fn try_server_only(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);
    let Some(owner) = dsl.get_owner_identity_by_id(&0) else {
        return Err("Owner identity not found".to_string());
    };

    if ctx.sender == owner.owner_ident {
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


/*  get_player_identity_by_username(ctx: &ReducerContext, username: &String) -> Option<Identity>
Change list :
01/06/2025 - KS - Initial Version
*/
/// Get players Identity by username. 
/// If the receiver is online, we get their identity from the online player list.
/// 
/// If the receiver is offline, we get their identity from the offline player list.
/// 
/// If the receiver does not exist, we return None.
pub fn get_player_identity_by_username(ctx: &ReducerContext, username: &String) -> Option<Identity> {
    let dsl = dsl(ctx);
    log::debug!("Looking up player identity for username: {}", username);

    let player_identity = match dsl.get_online_player_by_username(&username) {
        Some(player) => {
            log::debug!("Found online player: {} with identity: {:?}", username, player.identity);
            player.identity
        },
        None => {
            log::debug!("Player {} not found online, checking offline players.", username);
            match dsl.get_offline_player_by_username(&username) {
                Some(player) => {
                    log::debug!("Found offline player: {} with identity: {:?}", username, player.identity);
                    player.identity
                },
                None => {
                    log::debug!("Player {} not found online or offline.", username);
                    return None;
                }
            }
        },
    };
    Some(player_identity)
}