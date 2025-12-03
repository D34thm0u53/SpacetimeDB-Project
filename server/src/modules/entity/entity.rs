use spacetimedb::{table, ReducerContext, SpacetimeType};
use spacetimedsl::dsl;

use super::entity_position::*;
use super::entity_rotation::*;

/// What kind of entity it is.
#[derive(SpacetimeType, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum EntityType {
    Player,
    NonPlayer,
    World,
}

/// Core entity table - represents any entity in the world (players, NPCs, world objects).
/// Each entity has its own unique EntityId, with optional ownership by a PlayerAccount.
#[dsl(plural_name = entities,
    method(
        update = true,
        delete = true
    ),
    hook(after(insert))
)]
#[table(name = entity, public)]
pub struct Entity {
    #[primary_key]
    #[index(btree)]
    #[auto_inc]
    #[create_wrapper]
    #[referenced_by(path = super::entity_position, table = entity_position)]
    #[referenced_by(path = super::entity_rotation, table = entity_rotation)]
    #[referenced_by(path = super::entity_position, table = entity_chunk)]
    id: u32,

    /// Optional owner - links player-controlled entities to their PlayerAccount.
    /// Use 0 for world entities or NPCs without player ownership.
    #[index(btree)]
    pub owner_id: u32,

    /// The type of the entity.
    entity_type: EntityType,
}


/// Hook that automatically creates related records after Entity insertion.
/// Creates EntityPosition, EntityRotation, and EntityChunk records with default values.
#[spacetimedsl::hook]
fn after_entity_insert(
    dsl: &spacetimedsl::DSL,
    new_entity: &Entity,
) -> Result<(), spacetimedsl::SpacetimeDSLError> {
    let entity_id = new_entity.get_id();

    // Create the entity position record with default coordinates
    dsl.create_entity_position(CreateEntityPosition {
        id: entity_id.clone(),
        x: 0,
        y: 0,
        z: 0,
    })?;

    // Create the entity rotation record with default rotation
    dsl.create_entity_rotation(CreateEntityRotation {
        id: entity_id.clone(),
        rot_x: 0,
        rot_y: 0,
        rot_z: 0,
    })?;

    // Create the entity chunk record (starting at chunk 0,0)
    dsl.create_entity_chunk(CreateEntityChunk {
        id: entity_id,
        chunk_x: 0,
        chunk_z: 0,
    })?;

    Ok(())
}


/// Creates a new entity with the specified type and optional owner.
/// Automatically creates related EntityPosition, EntityRotation, and EntityChunk records
/// through the after_entity_insert hook.
/// 
/// # Arguments
/// * `ctx` - The reducer context
/// * `entity_type` - The type of entity to create (Player, NonPlayer, World)
/// * `owner_id` - PlayerAccountId linking this entity to a player (use 0 for no owner)
/// 
/// # Returns
/// The created Entity with its generated EntityId
pub fn create_entity_tree(
    ctx: &ReducerContext,
    entity_type: EntityType,
    owner_id: u32,
) -> Result<Entity, String> {
    let dsl = dsl(ctx);

    // Create a new entity - the hook will automatically create related records
    dsl.create_entity(CreateEntity {
        owner_id,
        entity_type,
    })
    .map_err(|e| format!("Failed to create entity: {:?}", e))
}




