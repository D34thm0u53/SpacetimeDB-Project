# SpacetimeDB Multiplayer Game Framework

A flexible multiplayer game framework built on [SpacetimeDB](https://spacetimedb.com), demonstrating real-time entity synchronization, spatial chunking, chat, and a friends management system. This framework can be adapted for multiple game types including FPS, RPG, strategy games, and more.


## 🎯 Project Purpose

This project serves as both a functional multiplayer game backend and a reference implementation for building real-time multiplayer games with SpacetimeDB. It showcases:

- **Real-time Entity Synchronization** – Player positions and rotations synchronized across clients through an intermediary table.
- **Spatial Chunk System** –  Chunking to enable proximity based position updates
- **Role-Based Permissions** – Secure admin tools and privilege escalation prevention
- **Chat & Social Systems** – Global chat, direct messaging, player ignore lists, and moderation tools
- **Health & Combat** – Damage application with shield/health logic

## 📋 Current Features

### ✅ Implemented
- Player management (connection, disconnection, username, audit logging)
- Roles and permissions system with audit trail
- Entity position and rotation tracking with spatial chunks
- Health/combat system (damage application, shield/health logic)
- Event & audit logging (player actions, system events, role changes)
- Authentication system with scheduled processing
- Global chat with archival system
- Direct messaging between players
- Player ignore/mute functionality

### 🚧 Planned
- Inventory/loadout system
- Weapons system (weapon stats, user weapons, weapon management)
- Friends system
- Scoring & leaderboards
- Progression & unlocks (XP, leveling, cosmetics)
- Anti-cheat & advanced security measures
- Server-side physics validation - likely via sidecar
- Customization & cosmetics system

### Key Patterns

- **Split Entity Tables** – Position and chunk data separated to reduce bandwidth
- **Online/Offline Tables** – Players moved between tables on connection events to optimize queries
- **Self-Scheduling Reducers** – Avoid timer creep with self-rescheduling pattern
- **SpacetimeDSL** – Ergonomic database operations with type safety

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines to maintain code quality and consistency.
Not all standards have been followed as closely as I should have. If you seen an improvement, point it out via an issue or raise a PR

#### Code Standards

**Formatting & Naming**
- Use `snake_case` for functions, variables, and parameters
- Limit function parameters to 3-4; use structs for more complex signatures
- Indent with 4 spaces
- Use `match` statements for enum control flow

**Documentation**
- Use `///` doc comments for public functions, structs, and enums
- Use `//` inline comments sparingly for complex logic
- Avoid comments that simply restate what code does
- Provide function summaries for complex reducers

**Error Handling**
- Use `Result<T, E>` for fallible functions
- Use the `?` operator for error propagation
- Log errors with sufficient context
- **Avoid `unwrap()` and `expect()`** – reducer panics rollback the entire transaction

**Logging Levels**
| Level | Usage |
|-------|-------|
| ERROR | System failures, database errors, critical issues |
| WARN  | Security violations (prefix with `SECURITY:`), failed operations |
| INFO  | Rare/important events (role changes, initialization) |
| DEBUG | Routine operations (connections, updates) |
| TRACE | Detailed flow control, validation attempts |

### Pull Request Process

1. **Fork** the repository and create a feature branch from `main`
2. **Follow** the coding standards outlined above
3. **Test** your changes with the unit-tester
4. **Update** documentation if adding new features or changing behavior
5. **Submit** a PR with a clear description of changes

### SpacetimeDB-Specific Guidelines

**Tables**
```rust
#[dsl(plural_name = my_table_entries)]
#[table(name = my_table, public)]
pub struct MyTable {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
    // ... fields
}
```

**Reducers**
- Always handle the `ReducerContext` as the first parameter
- Return `Result<(), String>` for proper error propagation to clients
- Validate inputs early and return descriptive errors
- Check permissions before performing privileged operations

**Scheduled Reducers**
- Use self-scheduling pattern to avoid timer drift
- Include security checks: `if ctx.sender != ctx.identity() { return Err(...) }`

### What We're Looking For

- Bug fixes and improvements to existing systems
- Implementation of planned features (see [todo.md](todo.md))
- Performance optimizations
- Documentation improvements
- Test coverage expansion

---

## 📄 License

This project is licensed under the Apache License 2.0 – see the [LICENSE](LICENSE) file for details.

## 🔗 Resources

- [SpacetimeDB Documentation](https://spacetimedb.com/docs)
- [SpacetimeDB Discord](https://spacetimedb.com/community)
- [Rust Book](https://doc.rust-lang.org/book/)

---

*Built using [SpacetimeDB](https://spacetimedb.com)*

