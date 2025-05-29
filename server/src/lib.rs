use spacetimedb::{ReducerContext};
use spacetimedb::{reducer};
use spacetimedsl::dsl;


pub mod modules;
use modules::player::*;
use modules::roles::*;

pub mod schedulers;
use schedulers::scheduler_chunks::{self};


#[reducer(init)]
// Called when a client connects to a SpacetimeDB database server
fn database_init(ctx: &ReducerContext) {

    
    scheduler_chunks::init(ctx)
        .expect("Failed to initialize chunk scheduler");
    // Initialize the database
    let dsl = dsl(ctx);
    // Create the player table if it doesn't exist
    dsl.create_role(1, ctx.identity(), false, false, false)
        .expect("Failed to create initial role");

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

