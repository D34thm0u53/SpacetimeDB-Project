
## Server Source Structure (`server/src/`)

### Core File

- **lib.rs**
  - Entry point for the server logic. Initializes the database and chunk scheduler on startup.
  - Handles client connection and disconnection events.
  - Imports modules for player and role management, and integrates the core scheduling logic.

### Modules (`server/src/modules/`)

- **mod.rs**
  - Module declaration file for organizing and exposing submodules within `modules/`.

- **admin_tools.rs**
  - Placeholder file. will contain a range of tools for modifying / manipulating another users profile and related records.
  - Planned features include:
    - Forced Name Changes
    - Modification of ignore lists
    - updates of enitity position for a player id
    - adding a player to a table used in rls to: block a user from recieving player positions. - soft player blocking

- **chats.rs**
  - Implements chat-related functionality.
  - Planned features include:
   - Report a chat message
   - Mark a chat message as removed - keep the record, but prevent it from being sent to a client via RLS.


- **common.rs**
  - Contains shared types, traits, or functions used by multiple modules.

- **entity_positions.rs**
  - Manages positions of entities within the game world. Handles storage, updates, and potentially queries for entity locations.
  - Tables split to reduce bandwidth. Once RLS is more mature, intention is to use the render_* fields to return only entity_position records, where the entity_chunk is within the render bounds.
  - Tables:
   - entity_position
    - Contains:
      - player_identity
      - x position
      - y position
      - z position
   - entity_chunk
    - Contains the
      - player_identity
      - chunk_x
      - chunk_z
        - Calculated on a 5 second scheduled reducer. Reducer is self scheduling to avoid creep.
      - render_topleft_x
      - render_topleft_z
      - render_bootomright_x 
      - render_bootomright_z
        - Calculated on an update call (called from the scheduled reducer) to be used in RLS once it is more mature.


- **player.rs**
  - Handles player logic, such as connection events, player data, and game-related actions.
  - Tables:
    - online_player
    - offline_player
  - players are moved from one table to the next at login/logoff events to avoind an additional step of filtering on online players for multiple queries.
  
- **roles.rs**
  - Manages user roles, permissions, and role-related behaviors within the game or server environment.
  - Defined roles exist to attempt to avoid a user gaining priv escalation.
  - Role change reducers to only be allowed by server or exisiting server admins

- **util.rs**
  - Utility functions and helpers used throughout the modules.

- **uuid.rs**
  - Functions for generating and handling UUIDs for unique identifiers within the server.
  - Presently unused as DSL does not support UUID in tables for unique or index fields.

### Schedulers (`server/src/schedulers/`)

- **mod.rs**
  - Module declaration file for organizing and exposing submodules within `schedulers/`.

- **scheduler_chunks.rs**
  - Implements chunk scheduling logic, likely for dividing the game world or server resources into manageable "chunks" and handling their scheduling (loading/unloading, updates, etc.).

