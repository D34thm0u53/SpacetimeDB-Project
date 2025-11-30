use spacetimedb::ReducerContext;
use spacetimedb::{reducer};


pub mod modules;
use modules::player::*;
use modules::*;

pub mod schedulers;


/// Initializes the database on first startup, loading default configurations.
#[reducer(init)]
fn database_init(ctx: &ReducerContext) {
    util::init_default_configs(ctx)
        .expect("Failed to initialize global configuration");
}

/// Handles client connection events, creating or updating player accounts.
#[reducer(client_connected)]
fn client_connected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, "connect");
}

/// Handles client disconnection events, moving players to offline status.
#[reducer(client_disconnected)]
fn client_disconnected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, "disconnect");
}

