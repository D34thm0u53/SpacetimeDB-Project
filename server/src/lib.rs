use spacetimedb::ReducerContext;
use spacetimedb::{reducer};
use spacetimedsl::dsl;


pub mod modules;
use modules::player::*;
use modules::roles::*;

pub mod schedulers;
use schedulers::*;

use modules::common::*;


#[reducer(init)]
// Called when a client connects to a SpacetimeDB database server
fn database_init(ctx: &ReducerContext) {
    let dsl = dsl(ctx);
    // initi the owner table
    create_owner_record(ctx)
        .expect("Failed to create owner record");
    
    scheduler_chunks::init(ctx)
        .expect("Failed to initialize chunk scheduler");

    scheduler_chat_archive::init(ctx)
        .expect("Failed to initialize chat archive timer");
    // Initialize the database

    // Create the player table if it doesn't exist
    dsl.create_role(1, ctx.identity(), false, false, false)
        .expect("Failed to create initial role");

    // Initialize default weapons
    crate::modules::weapon::initialize_default_weapons(ctx);
}

#[reducer(client_connected)]
// Called when a client connects to a SpacetimeDB database server
fn client_connected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, 1);
}

#[reducer(client_disconnected)]
// Called when a client disconnects from SpacetimeDB database server
fn client_disconnected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, 2);
}

