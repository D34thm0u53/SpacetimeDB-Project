use spacetimedb::{ReducerContext};

use spacetimedsl::dsl;


use crate::modules::roles::*;
use crate::modules::player::*;
use crate::modules::entity_position::*;



fn is_admin_tools_authorized(ctx: &ReducerContext) -> bool {
    // Check if the sender is a game admin or server administrator
    let dsl = dsl(ctx);
    if let Some(roles) = dsl.get_role_by_user_identity(&ctx.sender) {
        roles.is_game_admin || roles.is_server_administrator
    } else {
        log::warn!("User: {:?}, attempted to run admin tools.", ctx.sender);
        false
    }
}

#[spacetimedb::reducer]
pub fn cleanup_inactive_players(ctx: &ReducerContext) {
    // Authorization check: Ensure the caller is a game admin or server admin
    // if !is_admin_tools_authorized(ctx) ||  try_server_only(ctx).is_err(){
    //     log::warn!("Unauthorized attempt to cleanup inactive players by {:?}", ctx.sender);
    //     return;
    // }

    let dsl = dsl(ctx);

    // Get all online players
    let online_players = dsl.get_all_offline_players();

    // Iterate through online players and remove those who have been inactive for more than 30 days
    for player in online_players {
        dsl.delete_entity_position_by_player_identity(&player.identity);
        dsl.delete_entity_chunk_by_player_identity(&player.identity);
    }
}
