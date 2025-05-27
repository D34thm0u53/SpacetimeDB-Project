mod module_bindings;
use module_bindings::*;

use spacetimedb_sdk::{credentials, DbContext, Error, Identity};
// Add these dependencies to your Cargo.toml:
// ureq = "2"
// serde_json = "1"

fn main() {
    // Connect to the database
    let ctx: DbConnection = connect_to_db();

    // Spawn a thread, where the connection will process messages and invoke callbacks.
    ctx.run_threaded();

    // Handle CLI input
    user_input_loop(&ctx);
}

/// The URI of the SpacetimeDB instance hosting our chat database and module.
const HOST: &str = "http://10.1.1.236:3000";
// const HOST: &str = "https://maincloud.spacetimedb.com";

/// The database name we chose when we published our module.
const DB_NAME: &str = "multiuserpositions";

/// Load credentials from a file and connect to the database.
fn connect_to_db() -> DbConnection {
    DbConnection::builder()
        // Register our `on_connect` callback, which will save our auth token.
        .on_connect(on_connected)
        // Register our `on_connect_error` callback, which will print a message, then exit the process.
        .on_connect_error(on_connect_error)
        // Our `on_disconnect` callback, which will print a message, then exit the process.
        .on_disconnect(on_disconnected)
        // If the user has previously connected, we'll have saved a token in the `on_connect` callback.
        // In that case, we'll load it and pass it to `with_token`,
        // so we can re-authenticate as the same `Identity`.
        .with_token(creds_store().load().expect("Error loading credentials"))
        // Set the database name we chose when we called `spacetime publish`.
        .with_module_name(DB_NAME)
        // Set the URI of the SpacetimeDB host that's running our database.
        .with_uri(HOST)
        // Finalize configuration and connect!
        .build()
        .expect("Failed to connect")
}

fn creds_store() -> credentials::File {
    credentials::File::new("multiuserpositions")
    // credentials::File::new("maincloud_multiuserpositions")
}

/// Our `on_connect` callback: save our credentials to a file.
fn on_connected(_ctx: &DbConnection, _identity: Identity, token: &str) {
    if let Err(e) = creds_store().save(token) {
        eprintln!("Failed to save credentials: {:?}", e);
    }
}

/// Our `on_connect_error` callback: print the error, then exit the process.
fn on_connect_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Connection error: {:?}", err);
    std::process::exit(1);
}

/// Our `on_disconnect` callback: print a note, then exit the process.
fn on_disconnected(_ctx: &ErrorContext, err: Option<Error>) {
    match err {
        Some(err) => {
            eprintln!("Disconnected: [{}]", err);
            std::process::exit(1);
        }
        None => {
            println!("Disconnected gracefully.");
            // Perform any necessary cleanup here
            // For example: ctx.cleanup() or similar logic if applicable
            std::process::exit(0); // Optionally replace this with a return or other logic
        }
    }
}


/// Read each line of standard input, and either set our name or send a message as appropriate.
fn user_input_loop(ctx: &DbConnection) {
    for line in std::io::stdin().lines() {
        let Ok(line) = line else {
            panic!("Failed to read from stdin.");
        };
        if let Some(_cmd) = line.strip_prefix("/") {
            if let Some(username) = line.strip_prefix("/setname " ) {
                if let Err(e) = ctx.reducers.set_username(username.to_string()) {
                    eprintln!("Error setting user name: {:?}", e);
                }
            }
            if let Some(username) = line.strip_prefix("/ignore " ) {
                if let Err(e) = ctx.reducers.ignore_target_player(username.to_string()) {
                eprintln!("Error setting user name: {:?}", e);
                }
            }
            if let Some(username) = line.strip_prefix("/unignore " ) {
                if let Err(e) = ctx.reducers.unignore_target_player(username.to_string()) {
                eprintln!("Error setting user name: {:?}", e);
                }
            }
            if let Some(args) = line.strip_prefix("/setpos ") {
                // Parse three floats from the args
                let parts: Vec<&str> = args.split_whitespace().collect();
                if parts.len() == 3 {
                    let x = parts[0].parse::<f32>();
                    let y = parts[1].parse::<f32>();
                    let z = parts[2].parse::<f32>();
                    match (x, y, z) {
                        (Ok(x), Ok(y), Ok(z)) => {
                            if let Some(player_identity) = ctx.try_identity() {
                                let pos = module_bindings::EntityPosition {
                                    player_identity,
                                    x,
                                    y,
                                    z,
                                };
                                if let Err(e) = ctx.reducers.update_my_position(pos) {
                                    eprintln!("Error updating position: {:?}", e);
                                }
                            } else {
                                eprintln!("Could not determine your player identity.");
                            }
                        },
                        _ => {
                            eprintln!("Usage: /setpos <x> <y> <z> (all floats)");
                        }
                    }
                } else {
                    eprintln!("Usage: /setpos <x> <y> <z> (all floats)");
                }
            }
            if let Some(args) = line.strip_prefix("/circle ") {
                // this command will send position updates to the server to continuously move in a circle
                // Usage: /circle <radius> <speed>
                let parts: Vec<&str> = args.split_whitespace().collect();
                if parts.len() == 2 {
                    let radius = parts[0].parse::<f32>();
                    let speed = parts[1].parse::<f32>();
                    match (radius, speed) {
                        (Ok(radius), Ok(speed)) => {
                            if let Some(player_identity) = ctx.try_identity() {
                                // Spawn a thread to move in a circle
                                let mut angle = 0.0f32;
                                println!("Started moving in a circle with radius {} and speed {}", radius, speed);
                                loop {
                                    let x = radius * angle.cos();
                                    let y = 0.0;
                                    let z = radius * angle.sin();;
                                    let pos = module_bindings::EntityPosition { player_identity, x, y, z };
                                    if let Err(e) = ctx.reducers.update_my_position(pos) {
                                        eprintln!("Error updating position: {:?}", e);
                                    }
                                    angle += speed * 0.1;
                                    if angle > std::f32::consts::TAU {
                                        angle -= std::f32::consts::TAU;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(10));
                                }
                                
                                
                            } else {
                                eprintln!("Could not determine your player identity.");
                            }
                        }
                        _ => {
                            eprintln!("Usage: /circle <radius> <speed> (both floats)");
                        }
                    }
                } else {
                    eprintln!("Usage: /circle <radius> <speed> (both floats)");
                }
                
            }
        else if let Some(message) = line.strip_prefix("") {
            if let Err(e) = ctx.reducers.send_global_chat(message.to_string()) {
                eprintln!("Error sending message: {:?}", e);
            }
        } else {
            println!("Unknown command: {}", line);
        }
    }
    }}
