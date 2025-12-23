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


// Set this to your the OIDC client (or set of clients) set up for your
// SpacetimeAuth project.
const OIDC_CLIENT_ID: &str = "client_031CVDRbDed69EKkv8duSe";

#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) -> Result<(), String> {
    let jwt = ctx.sender_auth().jwt().ok_or("Authentication required".to_string())?;
    if jwt.issuer() != "https://auth.spacetimedb.com/oidc" {
        return Err("Invalid issuer".to_string());
    }

    if !jwt.audience().iter().any(|a| a == OIDC_CLIENT_ID) {
        return Err("Invalid audience".to_string());
    }
    handle_player_connection_event(ctx, "connect");
    Ok(())
}

/// Handles client disconnection events, moving players to offline status.
#[reducer(client_disconnected)]
fn client_disconnected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, "disconnect");
}

