# SpacetimeDB FPS Game - Copilot Instructions

## Project Overview
This is a SpacetimeDB-based multiplayer FPS game with a **client-server architecture** where:
- **Server** (`server/`) contains the SpacetimeDB module with reducers, tables, and game logic
- **Client** (`client/`) is a Rust-based test client that connects to the SpacetimeDB instance
- **SpacetimeDSL** is used extensively for ergonomic database operations and type safety

## Key Architecture Patterns

### SpacetimeDB Module Structure
- Entry point: `server/src/lib.rs` - defines lifecycle reducers (`init`, `client_connected`, `client_disconnected`)
- Modules in `server/src/modules/`: `player.rs`, `chat.rs`, `entity_positions.rs`, `roles.rs`, etc.
- Schedulers in `server/src/schedulers/`: self-scheduling reducers for recurring tasks like chat archiving

### SpacetimeDSL Usage Patterns
**Always use SpacetimeDSL attributes on tables:**
```rust
#[dsl(plural_name = global_chat_messages)]
#[table(name = global_chat_message, public)]
pub struct GlobalChatMessage {
    #[primary_key]
    #[auto_inc] 
    #[create_wrapper]
    id: u32,
    // ... other fields
}
```
- `#[create_wrapper]` on primary keys for type safety
- `#[dsl(plural_name = ...)]` for DSL method generation
- Use `dsl(ctx)` to get DSL instance, then `dsl.create_table_name()` for insertions

### Authentication & Identity Management
- Custom "So Stupid it Works" authentication flow using `lazy_static` mutex for identity tracking
- Guest users → authenticated users → online/offline player tables
- Identity tokens stored in client credentials file via `credentials::File::new("fps-base")`

### Data Flow Patterns
- **Client** calls reducers via `ctx.reducers().reducer_name()`
- **Server** processes in reducers, updates tables
- **Client** receives updates via subscriptions: `SELECT * FROM table_name`
- Split entity data across tables (`entity_position`, `entity_chunk`) to optimize bandwidth

## Development Workflows

### Building & Running
```bash
# Server: Build SpacetimeDB module
cd server && cargo build --target wasm32-unknown-unknown --release

# Client: Run test client (connects to HOST in main.rs)
cd client && cargo run

# Deploy module to SpacetimeDB
spacetime publish server/target/wasm32-unknown-unknown/release/Mouse-Game.wasm
```

### Code Generation
- Server changes require regenerating client bindings
- Generated files in `client/src/module_bindings/` - **never edit manually**
- Use `spacetime generate` to update client bindings after server changes

### Testing Approach
- Client includes comprehensive test suite in `run_reducer_tests()`
- Mock data creation pattern using HTTP requests to generate test identities
- Tests cover: chat system, entity updates, combat, admin functions
- Interactive CLI mode for manual testing

## Critical Dependencies
- **Server**: `spacetimedb = "1.3.0"` with `"unstable"` features, `spacetimedsl = "0.10.0"`
- **Client**: `spacetimedb-sdk = "1.3"` for connection management
- **Key Server Pattern**: `crate-type = ["cdylib"]` required for WASM compilation

## Entity & Spatial Systems
- Entity positions split across `entity_position` (x,y,z coordinates) and `entity_chunk` (spatial partitioning)
- Chunk system designed for future Row-Level Security (RLS) to limit spatial data bandwidth
- 5-second scheduled reducer updates chunk boundaries and render bounds

## Database Security Model
- Role-based permissions in `roles.rs` with `RoleType` enum
- Private tables for sensitive data (auth keys, internal game state)
- Public tables for shared game state (chat, entity positions, player accounts)
- Ignore system prevents blocked users' messages from being displayed

## Common Gotchas
- Always check `spacetimedb_sdk::Status` in reducer callbacks: `Committed`, `Failed(err)`, `OutOfEnergy`
- Use `.expect()` carefully in reducers - failures rollback entire transaction
- Client subscriptions need manual setup for each table you want to cache locally
- SpacetimeDSL requires specific attribute combinations - follow existing patterns

## Logging Standards
- **ERROR**: System failures, database errors, critical issues that require attention
- **WARN**: Security violations (prefix with "SECURITY:"), failed operations, unexpected states
- **INFO**: Rare or important events (role changes, system initialization)
- **DEBUG**: Routine operations (player connections, entity updates, successful transactions)
- **TRACE**: Detailed flow control and validation attempts
- Avoid logging every successful operation at INFO level to prevent log bloat