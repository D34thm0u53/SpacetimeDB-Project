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

use stdb_position_type::StdbPosition; // Import the correct type
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::f32::consts::PI;

lazy_static! {
    static ref CURRENT_TRANSFORM: Mutex<StdbTransform> = Mutex::new(StdbTransform {
        position: StdbPosition { x: 0.0, y: 0.0, z: 0.0 },
        rotation: StdbRotation { x: 0.0, y: 0.0, z: 0.0 },
    });
}

fn send_random_position(ctx: &DbConnection) {
    // Generate random deltas for position and rotation
    let delta_position = generate_random_position();
    let delta_rotation = generate_random_rotation();

    // Update the current transform stored in the static
    let mut current_transform = CURRENT_TRANSFORM.lock().unwrap();
    current_transform.position.x += delta_position.x;
    current_transform.position.y += delta_position.y;
    current_transform.position.z += delta_position.z;

    current_transform.rotation.x += delta_rotation.x;
    current_transform.rotation.y += delta_rotation.y;
    current_transform.rotation.z += delta_rotation.z;

    // Send the updated position and rotation to the database
    let _ = ctx.reducers.update_my_position(current_transform.clone());
}


fn generate_random_position() -> StdbPosition {
    let mut rng = rand::rng();
    StdbPosition {
        x: rng.random_range(-0.5..0.5),
        y: 0.0, // Fixed y value for simplicity
        //y: rng.random_range(-1.0..1.0),
        z: rng.random_range(-0.5..0.5),
        
    }
}

fn generate_random_rotation() -> StdbRotation {
    let _rng = rand::rng();
    StdbRotation {
        x:0.0,
        y:0.0,
        z:0.0,
        //x: rng.random_range(-1.0..10.0),
        //y: rng.random_range(-1.0..10.0),
        //z: rng.random_range(-1.0..10.0),
    }
}



fn send_my_position(ctx: &DbConnection) {
    // Generate new absolute position and rotation
    let new_position = generate_new_position();
    let new_rotation = generate_new_rotation();

    // Update the current transform stored in the static
    let mut current_transform = CURRENT_TRANSFORM.lock().unwrap();
    current_transform.position = new_position;
    current_transform.rotation = new_rotation;

    // Send the updated position and rotation to the database
    let _ = ctx.reducers.update_my_position(current_transform.clone());
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
        x,
        y: 0.0, // Fixed y value for simplicity
        z,
    }
}

fn generate_new_rotation() -> StdbRotation {
    let mut _rng = rand::rng();
    StdbRotation {
        x:0.0,
        y:0.0,
        z:0.0,
        //x: rng.random_range(-1.0..10.0),
        //y: rng.random_range(-1.0..10.0),
        //z: rng.random_range(-1.0..10.0),
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
            loop {
                send_my_position(ctx);
                std::thread::sleep(std::time::Duration::from_millis(1000/500));
                println!("Looping...");
            }

        }
    }
}