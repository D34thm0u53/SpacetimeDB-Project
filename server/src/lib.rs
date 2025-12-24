/// OIDC Token Type Note:
///
/// - **Access Token**: Used for authorization to access protected resources.
///   It may not contain the same claim structure and is not appropriate for
///   this authentication validation.
///
/// - **ID Token**: Contains identity claims (issuer, audience, subject) and is
///   intended for authentication. This is what SpacetimeAuth provides and what
///   should be validated here.
///
/// The validation logic checks `issuer()` and `audience()` claims, which are
/// standard ID token claims issued by the OIDC provider at
/// `https://auth.spacetimedb.com/oidc`.
use spacetimedb::ReducerContext;
use spacetimedb::{reducer};


pub mod modules;
use modules::player::*;
use modules::util::log_security_audit;
use modules::*;

pub mod schedulers;


/// Initializes the database on first startup, loading default configurations.
#[reducer(init)]
fn database_init(ctx: &ReducerContext) {
    util::init_default_configs(ctx)
        .expect("Failed to initialize global configuration");
}


// Set this to your OIDC client (or set of clients) set up for your
// SpacetimeAuth project. You can override this default at compile time by
// setting the `OIDC_CLIENT_ID` environment variable, e.g.:

/// Called whenever a new client connection is established.
///
/// This lifecycle reducer enforces SpacetimeAuth-based authentication for
/// incoming connections. The client must present a valid JWT issued by
/// `https://auth.spacetimedb.com/oidc` whose audience list includes the
/// configured OIDC client ID (`OIDC_CLIENT_ID`).
///
/// If the JWT is missing or fails validation (issuer or audience mismatch),
/// this reducer returns an error and the host disconnects the client. On
/// success, the connection is accepted and `handle_player_connection_event`
/// is invoked to perform any additional player-specific initialization.
const OIDC_CLIENT_ID: &str = match option_env!("OIDC_CLIENT_ID") {
    Some(id) => id,
    None => "client_031CVDRbDed69EKkv8duSe",
};



#[reducer(client_connected)]
fn client_connected(ctx: &ReducerContext) -> Result<(), String> {
    // Verify OIDC authentication if available
    let auth = ctx.sender_auth();


    let client_jwt = auth.jwt();

    match client_jwt {
        None => {
            spacetimedb::log::warn!("No JWT token provided by client");
            let _ = log_security_audit(
                ctx,
                &format!(
                    "Client connection rejected - missing JWT token. Client: {}",
                    ctx.sender
                ),
            );
            return Err("Missing JWT token".to_string());
        }
        Some(jwt) => {
            spacetimedb::log::trace!("Validating JWT token");

            
            match jwt.issuer() {
                issuer @ "https://auth.spacetimedb.com" => {
                    spacetimedb::log::trace!("JWT contains issuer used by SpacetimeDB Maincloud Dashboard: {}", issuer);
                }
                issuer @ "https://auth.spacetimedb.com/oidc" => {
                    spacetimedb::log::trace!("JWT Issuer: {}", issuer);
                }
                _ => {
                    let _ = log_security_audit(
                        ctx,
                        &format!(
                            "Client connection rejected - invalid OIDC issuer. Client: {}, Issuer: {}",
                            ctx.sender,
                            jwt.issuer()
                        ),
                    );
                    return Err("Invalid issuer".to_string());
                }
            }

            if !jwt.audience().iter().any(|a: &String| a == OIDC_CLIENT_ID || a == "spacetimedb") {
                let _ = log_security_audit(
                    ctx,
                    &format!(
                        "Client connection rejected - invalid audience claim. Client: {}, Raw: {}",
                        ctx.sender,
                        jwt.raw_payload()
                    ),
                );
                return Err("Invalid audience".to_string());
            }
            spacetimedb::log::trace!(
                "Client authenticated successfully via OIDC. Client: {}",
                ctx.sender
            );
        }
    }
   

    handle_player_connection_event(ctx, "connect");
    Ok(())
}

/// Handles client disconnection events, moving players to offline status.
#[reducer(client_disconnected)]
fn client_disconnected(ctx: &ReducerContext) {
    handle_player_connection_event(ctx, "disconnect");
}

