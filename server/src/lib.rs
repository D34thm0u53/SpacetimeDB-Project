use spacetimedb::ReducerContext;
use spacetimedb::{reducer};


pub mod modules;
use modules::player::*;
use modules::*;

pub mod schedulers;
use schedulers::*;



#[reducer(init)]
// Called when a client connects to a SpacetimeDB database server
fn database_init(ctx: &ReducerContext) {
    // scheduler_chunks::init(ctx)
    //     .expect("Failed to initialize chunk scheduler");

    scheduler_chat_archive::init(ctx)
        .expect("Failed to initialize chat archive timer");

    // Initialize the database
    // Authentication is now handled by SpaceTimeAuth

    // Initialize default weapons
    // crate::modules::weapon::initialize_default_weapons(ctx);
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

