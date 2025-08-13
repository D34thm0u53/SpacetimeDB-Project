use spacetimedb::{table, Identity, ReducerContext, reducer ,Timestamp};

use spacetimedsl::{ dsl, Wrapper };
use log::*;

use crate::modules::player::*;


#[reducer]
pub fn build_mock_data(ctx: &ReducerContext, mock_identity: Identity, mock_username: String)-> Result<(), String> {
    let dsl = dsl(ctx);
    // check if this identity or username already exist.
    // If so, delete the existing record, and create a new one.

    if dsl.get_player_account_by_identity(&mock_identity).is_ok() {
        dsl.delete_player_account_by_identity(&mock_identity)?;
    }
    if dsl.get_player_account_by_username(&mock_username).is_ok() {
        dsl.delete_player_account_by_username(&mock_username)?;
    }

    match create_player_account_and_online(ctx, mock_identity, mock_username) {
        Ok((player_account, online_player)) => {
            log::info!("Created new PlayerAccount: {:?}", player_account);
            log::info!("Created new OnlinePlayer: {:?}", online_player);
            Ok(())
        },
        Err(e) => {
            log::error!("Failed to create player account: {}", e);
            Err(e)  
        }
    }
}
