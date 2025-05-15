use spacetimedb::{table, reducer, Table, ReducerContext, Identity, Timestamp};

pub mod modules;

use modules::player::player_login;
use modules::player::player_logout;


#[reducer(client_connected)]
// Called when a client connects to a SpacetimeDB database server
pub fn client_connected(ctx: &ReducerContext) {
    player_login(ctx);
}

#[reducer(client_disconnected)]
// Called when a client disconnects from SpacetimeDB database server
pub fn client_disconnected(ctx: &ReducerContext) {
    player_logout(ctx);
}
