use spacetimedb::{table, Identity, ReducerContext, Timestamp};
use spacetimedsl::dsl;

use crate::modules::player::*;
use crate::modules::util::log_player_action_audit;

#[dsl(plural_name = global_chat_messages)]
#[table(name = global_chat_message, public)]
pub struct GlobalChatMessage {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    pub id: u64,

    pub identity: Identity, // FK to Player
    pub username: String,
    pub message: String,

    created_at: Timestamp,
}

#[dsl(plural_name = player_ignore_pairs)]
#[table(name = player_ignore_pair, public)]
pub struct PlayerIgnorePair {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub ignorer_identity: Identity, // FK to Player
    pub ignored_identity: Identity,
    created_at: Timestamp,
}

use spacetimedb::{client_visibility_filter, Filter};
#[client_visibility_filter]
const PLAYER_IGNORE_PAIR_FILTER: Filter = Filter::Sql(
    "SELECT * FROM player_ignore_pair WHERE ignorer_identity = :sender",
);

//// Impls ///



impl GlobalChatMessage {
    //
}

#[spacetimedb::reducer]
pub fn ignore_target_player(ctx: &ReducerContext, username: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Check if the target player is already ignored
    let target_identity = match dsl.get_player_by_username(&username) {
        Some(player) => player.identity,
        None => return Err("Target player not found.".to_string()),
    };
    // Check if the ignore pair already exists
    let already_ignored = dsl
        .get_all_player_ignore_pairs()
        .any(|pair| pair.ignorer_identity == ctx.sender && pair.ignored_identity == target_identity);
    if already_ignored {
        return Err("Player is already ignored.".to_string());
    }
    // Create a new ignore pair
    dsl.create_player_ignore_pair(ctx.sender, target_identity)?;
    // Log the ignore action
    log_player_action_audit(ctx, &format!("ignore:{}", target_identity));
    Ok(())
}

#[spacetimedb::reducer]
pub fn unignore_target_player(ctx: &ReducerContext, username: String) -> Result<(), String> {
    let dsl = dsl(ctx);
    // Check if the target player is already ignored
    let target_identity = match dsl.get_player_by_username(&username) {
        Some(player) => player.identity,
        None => return Err("Target player not found.".to_string()),
    };
    // Check if the ignore pair exists
    let _ignore_pair = match dsl
        .get_all_player_ignore_pairs()
        .find(|pair| pair.ignorer_identity == ctx.sender && pair.ignored_identity == target_identity) {
        Some(pair) => dsl.delete_player_ignore_pair_by_id(&pair.id),
        None => return Err("Player is not ignored.".to_string()),
    };
    // Log the unignore action
    log_player_action_audit(ctx, &format!("unignore:{}", target_identity));
    Ok(())
}


//// Reducers ///

#[spacetimedb::reducer]
pub fn send_global_chat(ctx: &ReducerContext, chat_message: String) -> Result<(), String> {
    let dsl = dsl(ctx);

    // If ctx.sender is a valid, unbanned, unmuted player
    dsl.create_global_chat_message(ctx.sender, &get_username(ctx, ctx.sender), &chat_message)?;
    Ok(())
}