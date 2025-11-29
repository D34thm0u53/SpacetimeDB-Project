use spacetimedb::ReducerContext;
use spacetimedb::{reducer};


pub mod modules;
use modules::player::*;
use modules::*;

pub mod schedulers;


#[reducer(init)]
// Called when a client connects to a SpacetimeDB database server
fn database_init(ctx: &ReducerContext) {
    // Initialize global configuration
    util::init_default_configs(ctx)
        .expect("Failed to initialize global configuration");

}

#[reducer(client_connected)]
// Called when a client connects to a SpacetimeDB database server
fn client_connected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, "connect");
}

#[reducer(client_disconnected)]
// Called when a client disconnects from SpacetimeDB database server
fn client_disconnected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, "disconnect");
}

