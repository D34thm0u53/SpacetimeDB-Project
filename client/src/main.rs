mod module_bindings;
use module_bindings::*;

use spacetimedb_sdk::{credentials, DbContext, Error, Identity, Table};
use std::thread;
use std::time::Duration;

// Global constants
const DB_NAME: &str = "fps-base";
const HOST: &str = "https://astro-mouse.org";
// const HOST: &str = "https://maincloud.spacetimedb.com";


// Entry point of the application
fn main() {
    println!("SpacetimeDB Reducer Test Client Starting...");
    
    // Connect to the database
    let ctx: DbConnection = connect_to_db();

    // Subscribe to SQL queries in order to construct a local partial replica of the database.
    subscribe_to_tables(&ctx);
    register_callbacks(&ctx);
    // Spawn a thread, where the connection will process messages and invoke callbacks.
    let handle = ctx.run_threaded();

    // Wait a moment for connection to stabilize
    thread::sleep(Duration::from_secs(2));

    // authenticate with the server
    authenticate(&ctx);
    
    // run_reducer_tests(&ctx);

    // Handle CLI input for manual testing
    println!("\nEntering interactive mode. Type 'help' for commands:");
    user_input_loop(&ctx);

    // Wait for the connection thread to finish
    handle.join().unwrap();
}

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
        // .with_token(creds_store().load().expect("Error loading credentials"))
        // Set the database name we chose when we called `spacetime publish`.
        .with_module_name(DB_NAME)
        // Set the URI of the SpacetimeDB host that's running our database.
        .with_uri(HOST)
        // Finalize configuration and connect!
        .build()
        .expect("Failed to connect")
}

fn creds_store() -> credentials::File {
    credentials::File::new("fps-base")
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

fn general_callbacks(ctx: &DbConnection) {
    let _apply_damage_callback_id = ctx.reducers().on_apply_damage(|ctx, victim, damage| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Applied {} damage to entity {:?}", damage, victim);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to apply damage: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to apply damage");
            }
        }
    });
}

fn subscribe_to_tables(ctx: &DbConnection) {
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .subscribe(["SELECT * FROM direct_message"]);
}

fn on_sub_applied(_ctx: &SubscriptionEventContext) {

}


fn register_callbacks(ctx: &DbConnection) {
    ctx.db.direct_message().on_insert(on_msg_inserted);
}

fn on_msg_inserted(ctx: &EventContext, msg: &DirectMessage) {
    // Get the current user's id (assuming it's available via ctx.identity())
    let my_id = ctx.identity();

    // Check if the message sender is ignored by the current user
    println!("Message via subscription {:?}:{:?}", msg.sender_id, msg.message);
}


/// Or `on_error` callback:
/// print the error, then exit the process.
fn on_sub_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Subscription failed: {}", err);
    std::process::exit(1);
}



fn authenticate(ctx: &DbConnection) {
    println!("🔐 Authenticating...");

    // Attempt to authenticate with the server
    match ctx.reducers().private_authenticate("this_is_a_test_auth_key".to_string()) {
        Ok(_) => {
            println!("Authentication request sent successfully.");
        }
        Err(e) => {
            println!("Failed to send authentication request: {}", e);
        }
    }

    // Wait a moment for authentication to complete
    thread::sleep(Duration::from_secs(2));
}



/// Read each line of standard input, and either set our name or send a message as appropriate.
fn user_input_loop(ctx: &DbConnection) {
    use std::io::{self, Write};
    
    // Print the current client's identity
    println!("Current identity: {}", ctx.identity().to_hex());


    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).unwrap() == 0 {
            break; // EOF
        }
        

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        
        match input {
            "name" | "set_name" => {
                print!("Enter your name: ");
                io::stdout().flush().unwrap();
                
                let mut name = String::new();
                if io::stdin().read_line(&mut name).unwrap() == 0 {
                    break; // EOF
                }
                
                let name = name.trim();
                if !name.is_empty() {
                    let _ = ctx.reducers().set_username(name.to_string());
                    println!("✅ Name set to '{}'", name);
                } else {
                    println!("❌ Name cannot be empty");
                }
            }
            ">" =>{
                print!("Enter message: ");
                io::stdout().flush().unwrap();

                let mut message = String::new();
                if io::stdin().read_line(&mut message).unwrap() == 0 {
                    break; // EOF
                }

                let message = message.trim();
                if !message.is_empty() {
                    match ctx.reducers().send_private_chat("stdb_admin".to_string(), message.to_string()) {
                        Ok(_) => println!("📩 DM sent to player 'stdb_admin': '{}'", message),
                        Err(e) => println!("❌ Failed to send DM: {}", e),
                    }
                } else {
                    println!("❌ Message cannot be empty");
                }

                // let message = message.trim();
                // if !message.is_empty() {
                //     match ctx.reducers().send_private_chat("c200cb4eb9c5a3cc8133e5c13aef".to_string(), message.to_string()) {
                //         Ok(_) => println!("📩 DM sent to player 'c200cb4eb9c5a3cc8133e5c13aef': '{}'", message),
                //         Err(e) => println!("❌ Failed to send DM: {}", e),
                //     }
                // } else {
                //     println!("❌ Message cannot be empty");
                // }

            }

            _ => {
                println!("❓ Unknown command.");
            }
        }
    }
    
    println!("👋 Goodbye!");
}


