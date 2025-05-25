mod module_bindings;
use module_bindings::*;

use spacetimedb_sdk::{credentials, DbContext, Error, Identity, Table, TableWithPrimaryKey};


fn main() {
    // Connect to the database
    let ctx: DbConnection = connect_to_db();

    // Register callbacks to run in response to database events.
    register_callbacks(&ctx);

    // Subscribe to SQL queries in order to construct a local partial replica of the database.
    subscribe_to_tables(&ctx);

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
    credentials::File::new("readerclient")
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




fn subscribe_to_tables(ctx: &DbConnection) {
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .subscribe(["SELECT * FROM global_chat_message"]);
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .subscribe(["SELECT * FROM player_ignore_pair"]);
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .subscribe(["SELECT * FROM entity_position"]);
}


fn on_sub_applied(_ctx: &SubscriptionEventContext) {
    println!("Fully connected and all subscriptions applied.");
    println!("Use /name to set your name, or type a message!");
}

/// Or `on_error` callback:
/// print the error, then exit the process.
fn on_sub_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Subscription failed: {}", err);
    std::process::exit(1);
}

fn register_callbacks(ctx: &DbConnection) {
    // When a new user joins, print a notification.

    ctx.db.global_chat_message().on_insert(on_msg_inserted);

    ctx.db.entity_position().on_update(on_entity_position_updated);

    // When we fail to set our name, print a warning.

}

fn on_msg_inserted(ctx: &EventContext, msg: &GlobalChatMessage) {
    // Get the current user's id (assuming it's available via ctx.identity())
    let my_id = ctx.identity();

    // Check if the message sender is ignored by the current user
    let is_ignored = ctx
        .db
        .player_ignore_pair()
        .iter()
        .any(|pair| pair.ignorer_identity == my_id && pair.ignored_identity == msg.identity);

    if !is_ignored {
        println!("{:?}:{:?}", msg.username, msg.message);
    }
}


fn on_entity_position_updated(ctx: &EventContext, _old_pos: &EnityPosition, new_pos: &EnityPosition) {
    // Get the current user's id (assuming it's available via ctx.identity())
    let my_id = ctx.identity();
    if new_pos.player_identity != my_id {
        println!("{:?}", new_pos);
    }
    
}


/// Read each line of standard input, and either set our name or send a message as appropriate.
fn user_input_loop(ctx: &DbConnection) {
    for line in std::io::stdin().lines() {
        let Ok(line) = line else {
            panic!("Failed to read from stdin.");
        };
        if let Some(_cmd) = line.strip_prefix("/"){
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
                                let pos = module_bindings::StdbPosition { player_identity, x, y, z};
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
            } else {
                println!("Unknown command: {}", line);
            }
            
        }
    }
}