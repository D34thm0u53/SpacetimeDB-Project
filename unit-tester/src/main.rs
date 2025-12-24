mod module_bindings;
use module_bindings::*;
use spacetimedb_sdk::TableWithPrimaryKey;

use spacetimedb_sdk::{credentials, DbContext, Error, Identity, Table};
use std::thread;
use std::time::Duration;

use sha2::{Sha256, Digest};
use base64::{Engine, engine::general_purpose};
use std::sync::{Arc, Mutex};

// Global constants
const DB_NAME: &str = "mouse-game";
const HOST: &str = "https://maincloud.spacetimedb.com";

// OIDC Configuration
const CLIENT_ID: &str = "client_031CVDRbDed69EKkv8duSe";
const REDIRECT_URI: &str = "http://127.0.0.1:8080/callback";
const AUTH_URI: &str = "https://auth.spacetimedb.com/oidc/auth";
const TOKEN_URI: &str = "https://auth.spacetimedb.com/oidc/token";



// Entry point of the application
fn main(){
    println!("SpacetimeDB Reducer Test Client Starting...");
    
    // Before we connect to the db, we need to get our auth token.
    // Handle OIDC authentication if no saved credentials exist
    /* */
    let token = if let Ok(Some(saved_token)) = creds_store().load() {
        println!("Using saved authentication token");
        Some(saved_token)
    } else {
        println!("No saved credentials found. Starting OIDC authentication flow...");
        match get_auth_token() {
            Ok(token) => {
                println!("Successfully obtained authentication token");
                Some(token)
            }
            Err(e) => {
                eprintln!("Failed to authenticate: {}", e);
                std::process::exit(1);
            }
        }
    };
       


    // Connect to the database
    let ctx: DbConnection = connect_to_db(token);
    // Register callbacks to run in response to database events.
    register_callbacks(&ctx);

    // Subscribe to SQL queries in order to construct a local partial replica of the database.
    subscribe_to_tables(&ctx);

    // Spawn a thread, where the connection will process messages and invoke callbacks.
    let handle = ctx.run_threaded();

    // Wait a moment for connection to stabilize
    thread::sleep(Duration::from_secs(2));


    general_callbacks(&ctx);
    // Run automated tests for all reducers

    // run_reducer_tests(&ctx);

    // Handle CLI input for manual testing
    println!("\nEntering interactive mode. Type 'help' for commands:");
    user_input_loop(&ctx);

    // Wait for the connection thread to finish
    handle.join().unwrap();
}

/// Load credentials from a file and connect to the database.
fn connect_to_db(token: Option<String>) -> DbConnection {
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
        .with_token(token)
        // Set the database name we chose when we called `spacetime publish`.
        .with_module_name(DB_NAME)
        // Set the URI of the SpacetimeDB host that's running our database.
        .with_uri(HOST)
        // Finalize configuration and connect!
        .build()
        .expect("Failed to connect")
}

fn creds_store() -> credentials::File {
    credentials::File::new("mouse-game")
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

    let _set_player_roles_callback_id = ctx.reducers().on_set_player_roles(|ctx, target_identity, requested_role| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Set player role to {:?} for {:?}", requested_role, target_identity);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to set player roles (expected): {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to set player roles");
            }
        }
    });

    let _update_rotation_callback_id = ctx.reducers().on_update_my_rotation(|ctx, _entity, _old_rotation, new_rotation| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Rotation updated to ({}, {}, {})", 
                         new_rotation, new_rotation, new_rotation);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to update rotation: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to update rotation");
            }
        }
    });

    let _update_position_callback_id = ctx.reducers().on_update_my_position(|ctx, new_position| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Position updated to ({}, {}, {})", 
                         new_position.x, new_position.y, new_position.z);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to update position: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to update position");
            }
        }
    });

    let _ignore_player_callback_id = ctx.reducers().on_ignore_player(|ctx, target_identity| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Player ignored: {:?}", target_identity);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to ignore player: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to ignore player");
            }
        }
    });
    

    let _unignore_player_callback_id = ctx.reducers().on_unignore_player(|ctx, target_identity| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Player unignored: {:?}", target_identity);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to unignore player: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to unignore player");
            }
        }
    });

    let _send_private_chat_callback_id = ctx.reducers().on_send_private_chat(|ctx, target_username, message| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Private message sent to '{}': '{}'", target_username, message);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to send private message: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to send private message");
            }
        }
    });

    let _send_global_chat_callback_id = ctx.reducers().on_send_global_chat(|ctx, chat_message| {
        match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => {
                println!("Global chat message sent successfully: '{}'", chat_message);
            }
            spacetimedb_sdk::Status::Failed(err) => {
                println!("Failed to send global chat: {}", err);
            }
            spacetimedb_sdk::Status::OutOfEnergy => {
                println!("Out of energy to send global chat message");
            }
        }
    });


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
        .subscribe(["SELECT * FROM nearby_entity_chunks"]);
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .subscribe(["SELECT * FROM player_account"]);
    ctx.subscription_builder()
        .on_applied(on_sub_applied)
        .on_error(on_sub_error)
        .subscribe(["SELECT * FROM entity"]);
}


fn on_sub_applied(_ctx: &SubscriptionEventContext) {

}

/// Or `on_error` callback:
/// print the error, then exit the process.
fn on_sub_error(_ctx: &ErrorContext, err: Error) {
    eprintln!("Subscription failed: {}", err);
    std::process::exit(1);
}

fn register_callbacks(ctx: &DbConnection) {
    // When a new user joins, print a notification.

    ctx.db.global_chat_message().on_update(on_msg_inserted);
    // Views do not yet support primary keys, so we use on_insert here.
    ctx.db.nearby_entity_chunks().on_insert(on_chunk_calculation_updated);


    // When we fail to set our name, print a warning.

}

fn on_chunk_calculation_updated(_ctx: &EventContext, new_chunk: &EntityChunk) {
    // Get the current user's id (assuming it's available via ctx.identity())
    println!("Chunk updated for entity ID {:?}: chunk_x={}, chunk_z={}", new_chunk.id, new_chunk.chunk_x, new_chunk.chunk_z);
}



fn on_msg_inserted(ctx: &EventContext, _old_msg: &GlobalChatMessage, new_msg: &GlobalChatMessage) {
    // Get the current user's id (assuming it's available via ctx.identity())
    let my_id = ctx.identity();

    // Check if the message sender is ignored by the current user
    let is_ignored = ctx
        .db
        .player_ignore_pair()
        .iter()
        .any(|pair| pair.ignorer_identity == my_id && pair.ignored_identity == new_msg.identity);

    if !is_ignored {
        println!("{:?}:{:?}", new_msg.username, new_msg.message);
    }
}

/// Comprehensive test suite for all reducers based on API specification
fn run_reducer_tests(ctx: &DbConnection) {
    println!("Starting Comprehensive Reducer Tests...\n");
    println!("=================================================");
    test_chat_system(ctx);
    println!("=================================================");
    test_ignore_system(ctx);
    println!("=================================================");
    test_entity_system(ctx);
    println!("=================================================");
    test_combat_system(ctx);
    println!("=================================================");
    test_admin_system(ctx);
    println!("=================================================");
    println!("All reducer tests completed!\n");
}

/// Test Chat System Reducers
fn get_mock_identity() -> Option<Identity> {
    // Generate initial server identity and record response
    let client = reqwest::blocking::Client::new();
    let identity_response = client
        .post(format!("{}/v1/identity", HOST))
        .send();

    match identity_response {
        Ok(response) => {            
            match response.text() {
                Ok(body) => {
                    // Try to parse the JSON response to extract identity
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(identity_str) = json_value.get("identity").and_then(|v| v.as_str()) {
                            // Try to parse the identity string into an Identity type
                            match Identity::from_hex(identity_str) {
                                Ok(identity) => {
                                    println!(" Successfully parsed identity: {:?}", identity);
                                    // You can store or use the identity here if needed
                                    Some(identity)
                                }
                                Err(e) => {
                                    println!(" Failed to parse identity from hex: {}", e);
                                    println!(" Identity: {}", identity_str);
                                    None
                                }
                            }
                        } else {
                            println!(" No identity field found in response");
                            None
                        }
                    } else {
                        println!(" Failed to parse response as JSON");
                        None
                    }

                }
                Err(e) => {
                    println!(" Failed to read response body: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            println!(" Failed to make identity request: {}", e);
            return None
        }
    }
    
}

fn create_mock_data(ctx: &DbConnection, username: String)-> (Identity, String) {
    println!(" Generating a Mock Identity");
    // Create a dummy identity for testing (in a real scenario, this would be another player's identity)
    let mock_identity = get_mock_identity().expect("Failed to get mock identity");
    println!("");

    println!(" Generating a Mock User");
    ctx.reducers().build_mock_data(mock_identity, username.to_string()).expect("Failed to get mock identity");
    thread::sleep(Duration::from_millis(500));
    println!("");
    
    return (mock_identity, username.to_string());
}

/// Test Chat System Reducers
fn test_chat_system(ctx: &DbConnection) {
    println!(">> Testing Chat System Reducers...");
    
    println!("");
    let (_mock_identity, mock_username) = create_mock_data(ctx, "Chat_System_Test".to_string());

    // Test send_global_chat
    test_send_global_chat(ctx, &format!("Test message from client!"));
    thread::sleep(Duration::from_millis(500));
    
    // Test send_private_chat (assuming at least one other player exists)
    test_send_private_chat(ctx, &mock_username, "This is a private test message");
    thread::sleep(Duration::from_millis(500));

    //let _ = ctx.reducers().clear_mock_data(mock_identity, mock_username);
    println!("<<   Chat system tests completed\n");
}

fn test_ignore_system(ctx: &DbConnection) {
    println!(">> Testing Player ignore functions...\n");
    

    let (mock_identity, _mock_username) = create_mock_data(ctx, "Ignore_System_Test".to_string());

    println!("");
    println!("  Testing ignore_player");

    
    let _ = ctx.reducers().ignore_player(mock_identity);
    thread::sleep(Duration::from_millis(500));
    
    
    println!("");
    println!("  Testing unignore_player...");
    

    let _ = ctx.reducers().unignore_player(mock_identity);
    thread::sleep(Duration::from_millis(500));
    println!("");

    //let _ = ctx.reducers().clear_mock_data(mock_identity, mock_username);
    println!("<<   Player ignore tests completed\n");
}

/// Test Entity System Reducers  
fn test_entity_system(ctx: &DbConnection) {
    println!(">> Testing Entity System Reducers...");
    println!("");
    let (mock_identity, _mock_username) = create_mock_data(ctx, "PositionUpdate_Test".to_string());
    
    // Get player_account record for mock_identity
    let player_account = ctx.db.player_account()
        .identity()
        .find(&mock_identity)
        .expect("Mock player account should exist");
    
    // Wait for entity to be created by server hook
    thread::sleep(Duration::from_millis(500));
    
    // Find the entity owned by this player
    let entity = ctx.db.entity()
        .iter()
        .find(|e| e.owner_id == player_account.id)
        .expect("Entity should exist for player");

    // Test update_my_position
    test_update_position(ctx, 1024, 512, 256, entity.clone());
    thread::sleep(Duration::from_millis(500));
    println!("");
    // Test update_my_rotation
    test_update_rotation(ctx, 45, 90, 0);
    thread::sleep(Duration::from_millis(500));
    println!("");
    // Test multiple position updates
    test_update_position(ctx, 2048, 1024, 512, entity.clone());
    thread::sleep(Duration::from_millis(500));
    println!("");
    

    //let _ = ctx.reducers().clear_mock_data(mock_identity, mock_username);
    println!("<<   Entity system tests completed\n");
}

/// Test Combat System Reducers
fn test_combat_system(ctx: &DbConnection) {
    println!(">> Testing Combat System Reducers...");
    println!("");

    let (mock_identity, _mock_username) = create_mock_data(ctx, "Combat_Test".to_string());
    
    // Get player_account record for mock_identity
    let player_account = ctx.db.player_account()
        .identity()
        .find(&mock_identity)
        .expect("Mock player account should exist");
    
    // Test apply_damage (using a test entity ID)
    test_apply_damage(ctx, player_account.id, 100);
    thread::sleep(Duration::from_millis(500));
    println!("");
    // Test different damage amounts
    test_apply_damage(ctx, player_account.id, 1500);
    thread::sleep(Duration::from_millis(500));
    println!("");
    //let _ = ctx.reducers().clear_mock_data(mock_identity, mock_username);
    println!("<<   Combat system tests completed\n");
}

/// Test Administration System Reducers
fn test_admin_system(ctx: &DbConnection) {
    println!(">> Testing Administration System Reducers...");
    println!("");
    
    // Test set_player_roles (this will likely fail due to permissions)
    test_set_player_roles(ctx);
    thread::sleep(Duration::from_millis(500));
    println!("");
    println!("<<   Admin system tests completed\n");
}

/// Test individual reducers with proper parameters

fn test_send_global_chat(ctx: &DbConnection, message: &str) {
    println!("Testing send_global_chat with message: '{}'", message);
    
    let _ = ctx.reducers().send_global_chat(message.to_string());
    thread::sleep(Duration::from_millis(100));
    println!("");
    // Register callback to see the result
    
}

fn test_send_private_chat(ctx: &DbConnection, target_username: &str, message: &str) {
    println!("Testing send_private_chat to '{}': '{}'", target_username, message);
    
    let _ = ctx.reducers().send_private_chat(target_username.to_string(), message.to_string());
    thread::sleep(Duration::from_millis(500));
    println!("");
}



fn test_update_position(ctx: &DbConnection, x: i32, y: i32, z: i32, entity: Entity) {
    println!("Testing update_my_position to ({}, {}, {})", x, y, z);
    println!("");
    
    let new_position = EntityPosition {
        id: entity.id,
        x,
        y,
        z,
    };

    let _ = ctx.reducers().update_my_position(new_position);
    
    
}

fn test_update_rotation(ctx: &DbConnection, rot_x: i16, rot_y: i16, rot_z: i16) {
    let _ = ctx.reducers().update_my_rotation(rot_x, rot_y, rot_z);
}

fn test_apply_damage(ctx: &DbConnection, victim_id: u32, damage: u32) {
    println!("Testing apply_damage: {} damage to entity {}", damage, victim_id);
    
    let entity_id: PlayerAccountId = PlayerAccountId { value: victim_id };
    let _ = ctx.reducers().apply_damage(entity_id, damage);
    
}

fn test_set_player_roles(ctx: &DbConnection) {
    println!("Testing set_player_roles (likely to fail without admin permissions)");
    
    // Use our own identity as target (will likely fail)
    let target_identity = ctx.identity();
    let requested_role = RoleType::TrustedUser;
    
    let _ = ctx.reducers().set_player_roles(target_identity, requested_role.clone());
    
}


/// Read each line of standard input, and either set our name or send a message as appropriate.
fn user_input_loop(ctx: &DbConnection) {
    use std::io::{self, Write};
    
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
                    println!("Name set to '{}'", name);
                } else {
                    println!("Name cannot be empty");
                }
            }
            "help" | "h" => print_help(),
            "quit" | "q" | "exit" => break,
            "test" | "t" => run_reducer_tests(ctx),
            "status" | "s" => print_connection_status(ctx),
            "chat_test" => test_chat_system(ctx),
            "entity_test" => test_entity_system(ctx),
            "combat_test" => test_combat_system(ctx),
            "admin_test" => test_admin_system(ctx),
            _ if input.starts_with("chat ") => {
                let message = &input[5..];
                test_send_global_chat(ctx, message);
            }
            _ if input.starts_with("pm ") => {
                let parts: Vec<&str> = input[3..].splitn(2, ' ').collect();
                if parts.len() == 2 {
                    test_send_private_chat(ctx, parts[0], parts[1]);
                } else {
                    println!("Usage: pm <username> <message>");
                }
            }
            _ => {
                println!("Unknown command. Type 'help' for available commands.");
            }
        }
    }
    
    println!("Goodbye!");
}

/// Generate a cryptographic state for OIDC flow.
fn generate_state() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789-._~";
    let mut rng = rand::rng();
    
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate a code verifier for PKCE (minimum 43 characters).
fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789-._~";
    let mut rng = rand::rng();
    
    (0..128)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate code challenge for PKCE flow.
fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    general_purpose::URL_SAFE_NO_PAD.encode(&hash)
}

/// Start local HTTP server to handle OAuth redirect.
fn start_redirect_server(_state: Arc<Mutex<Option<String>>>) -> std::thread::JoinHandle<Option<String>> {
    thread::spawn(move || {
        let server = match tiny_http::Server::http("127.0.0.1:8080") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to start redirect server: {}", e);
                return None;
            }
        };

        println!("Waiting for authorization callback on http://127.0.0.1:8080/callback");

        for request in server.incoming_requests() {
            let path = request.url();
            
            let response = if path.starts_with("/callback") {
                if let Some(query) = path.split('?').nth(1) {
                    let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes())
                        .into_owned()
                        .collect();

                    if let Some(code) = params.get("code") {
                        let _ = request.respond(tiny_http::Response::from_string("Authorization successful! You can close this window."));
                        return Some(code.clone());
                    }
                    
                    if let Some(error) = params.get("error") {
                        let response_msg = format!("Error: {}", error);
                        let _ = request.respond(tiny_http::Response::from_string(&response_msg));
                        continue;
                    }
                }
                tiny_http::Response::from_string("Invalid callback parameters")
            } else {
                tiny_http::Response::from_string("Invalid request")
            };
            
            let _ = request.respond(response);
        }
        
        None
    })
}

/// Perform the OIDC authorization code flow.
fn get_auth_token() -> Result<String, Box<dyn std::error::Error>> {
    let state = generate_state();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Construct the authorization URL
    let encoded_redirect = urlencoding::encode(REDIRECT_URI);
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=openid%20profile%20email&state={}&code_challenge={}&code_challenge_method=S256",
        AUTH_URI, CLIENT_ID, encoded_redirect, state, code_challenge
    );

    println!("\nOIDC Authentication Debug Info:");
    println!("  CLIENT_ID: {}", CLIENT_ID);
    println!("  REDIRECT_URI: {}", REDIRECT_URI);
    println!("  Encoded Redirect: {}", encoded_redirect);
    println!("\nOpening browser for authorization...");
    println!("If browser doesn't open, visit this URL:\n{}", auth_url);
    
    // Try to open the URL in the default browser
    let _ = open_url(&auth_url);

    // Start the redirect server
    let state_holder = Arc::new(Mutex::new(None));
    let server_handle = start_redirect_server(state_holder.clone());

    // Wait for the authorization code
    let auth_code = server_handle.join()
        .ok()
        .flatten()
        .ok_or("Failed to get authorization code")?;

    println!("Received authorization code: {}", &auth_code[..auth_code.len().min(20)]);

    // Exchange the code for a token
    let client = reqwest::blocking::Client::new();
    println!("\nExchanging authorization code for token...");
    println!("  Token URI: {}", TOKEN_URI);
    println!("  Client ID: {}", CLIENT_ID);
    println!("  Redirect URI: {}", REDIRECT_URI);
    
    let token_response = client
        .post(TOKEN_URI)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &auth_code),
            ("client_id", CLIENT_ID),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", &code_verifier),
        ])
        .send()?;

    println!("Token response status: {}", token_response.status());
    
    let response_body = token_response.text()?;
    println!("Token response body: {}", response_body);

    let token_data: serde_json::Value = serde_json::from_str(&response_body)
        .map_err(|e| format!("Failed to parse token response as JSON: {}", e))?;
    
    println!("Parsed token response: {}", serde_json::to_string_pretty(&token_data)?);
    
    // Use id_token for SpacetimeDB authentication, NOT access_token.
    // The id_token is a JWT containing identity claims (issuer, audience, subject)
    // that SpacetimeDB uses to verify the client's identity.
    let id_token = token_data
        .get("id_token")
        .and_then(|v| v.as_str())
        .ok_or("No id_token in response")?
        .to_string();

    println!("Successfully obtained ID token");
    println!("ID token (first 20 chars): {}...", &id_token[..id_token.len().min(20)]);

    Ok(id_token)
}

/// Attempt to open a URL in the default browser.
fn open_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", url])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()?;
    }
    Ok(())
}

fn print_help() {
    println!("Available Commands:");
    println!("  help, h          - Show this help message");
    println!("  test, t          - Run all reducer tests");
    println!("  status, s        - Show connection status");
    println!("  chat_test        - Test chat system only");
    println!("  entity_test      - Test entity system only");
    println!("  combat_test      - Test combat system only");
    println!("  admin_test       - Test admin system only");
    println!("  chat <message>   - Send a global chat message");
    println!("  pm <user> <msg>  - Send a private message");
    println!("  quit, q, exit    - Exit the client");
}

fn print_connection_status(ctx: &DbConnection) {
    println!("Connection Status:");
    println!("  Identity: {:?}", ctx.identity());
    println!("  Connected: {}", ctx.is_active());
    
    // Print table statistics
    println!("Table Statistics:");
    println!("  Global Chat Messages: {}", ctx.db.global_chat_message().count());
    println!("  Entity Positions: {}", ctx.db.entity_position().count());
    println!("  Entity Rotations: {}", ctx.db.entity_rotation().count());
    println!("  Player Accounts: {}", ctx.db.player_account().count());
    println!("  Online Players: {}", ctx.db.online_player().count());
}