mod module_bindings;
use module_bindings::*;

use spacetimedb_sdk::{credentials, DbContext, Error, Identity, Table, TableWithPrimaryKey};
use rand::Rng;

fn main() {
    // Connect to the database
    let ctx: DbConnection = connect_to_db();

    // Register callbacks to run in response to database events.
    //register_callbacks(&ctx);

    // Subscribe to SQL queries in order to construct a local partial replica of the database.
    //subscribe_to_tables(&ctx);

    // Spawn a thread, where the connection will process messages and invoke callbacks.
    ctx.run_threaded();

    // Handle CLI input
    user_input_loop(&ctx);
}

/// The URI of the SpacetimeDB instance hosting our chat database and module.
const HOST: &str = "http://10.1.1.236:3000";

/// The database name we chose when we published our module.
const DB_NAME: &str = "multiuserpositions";
const MY_ID: u64 = 1;

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
            eprintln!("Disconnected: {}", err);
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

use std::sync::Mutex;
use lazy_static::lazy_static;



fn send_my_position(ctx: &DbConnection) {
    // Generate a random position and rotation
    let position = generate_new_position();

    // Send the PlayerEntity to the database
    if let Err(e) = ctx.reducers.update_my_position(position) {
        eprintln!("Error updating position: {:?}", e);
    }
}

fn generate_new_position() -> StdbPosition {

    // Define the circle's radius and the number of points
    const RADIUS: f32 = 12.0;
    const POINTS: usize = 180;

    // Static variable to keep track of the current angle
    lazy_static! {
        static ref CURRENT_ANGLE: Mutex<f32> = Mutex::new(0.0);
    }

    // Calculate the next position on the circle
    let mut angle = CURRENT_ANGLE.lock().unwrap();
    let x = RADIUS * angle.to_radians().cos();
    let z = RADIUS * angle.to_radians().sin();

    // Increment the angle for the next position
    *angle += 360.0 / POINTS as f32;
    if *angle >= 360.0 {
        *angle -= 360.0;
    }

    StdbPosition {
        player_id_fk: MY_ID,
        x,
        y: 0.0, // Fixed y value for simplicity
        z,
    }
}


/// Read each line of standard input, and either set our name or send a message as appropriate.
fn user_input_loop(ctx: &DbConnection) {
    for line in std::io::stdin().lines() {
        println!("Line input:{:?}", line);
        let Ok(line) = line else {
            panic!("Failed to read from stdin.");
        };
        if let Some(username) = line.strip_prefix("/setname " ) {
            if let Err(e) = ctx.reducers.set_user_name(username.to_string()) {
                eprintln!("Error setting user name: {:?}", e);
            }
        }
        if let Some(_username) = line.strip_prefix("/random" ) {
            send_my_position(ctx)
        }
        if let Some(user_role) = line.strip_prefix("/setrole ") {
            let role = match user_role {
                "user" => RoleType::User,
                "admin" => RoleType::GameAdmin,
                "trusted" => RoleType::TrustedUser,
                _ => {
                    eprintln!("Invalid role: {}", user_role);
                    continue;
                }
            };
            if let Err(e) = ctx.reducers.set_user_role(role ) {
                eprintln!("Error setting user role: {:?}", e);
            }
        }
        if let Some(_username) = line.strip_prefix("/exit" ) {
            if let Err(e) = ctx.disconnect() {
                eprintln!("Error disconnecting: {:?}", e);
            }
            break;
        }
        if let Some(_username) = line.strip_prefix("/validate" ) {
            if let Err(e) = ctx.reducers.validate_users() {
                eprintln!("Error validating users: {:?}", e);
            }

        }
    }
}