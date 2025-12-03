use spacetimedb::{table, Identity, ReducerContext, Timestamp};
use spacetimedsl::*;

// ============================================================================
// Player Audit System
// ============================================================================

/// Audit log for tracking player actions.
/// Records are immutable once created for compliance purposes.
#[dsl(plural_name = player_audits,
    method(
        update = false,
        delete = false
    )
)]
#[table(name = player_audit, public)]
pub struct PlayerAudit {
    #[primary_key]
    #[auto_inc]
    #[create_wrapper]
    id: u32,
    user_identity: Identity,
    action: String,
    created_at: Timestamp,
}

/// Logs a player action to the audit table.
///
/// # Arguments
/// * `ctx` - The reducer context
/// * `action` - Description of the action being logged
///
/// # Returns
/// Result indicating success or failure
pub fn log_player_action_audit(ctx: &ReducerContext, action: &str) -> Result<(), String> {
    let dsl = dsl(ctx);
    dsl.create_player_audit(CreatePlayerAudit {
        user_identity: ctx.sender,
        action: action.to_string(),
    })
    .map(|_| ())
    .map_err(|e| format!("Failed to create audit record: {:?}", e))
}

// ============================================================================
// Global Configuration System
// ============================================================================

/// Global configuration table for storing key-value pairs
/// Supports different value types through enum variants
#[dsl(plural_name = global_configs,
    method(
        update = true,
        delete = true
    )
)]
#[table(name = global_config, public)]
pub struct GlobalConfig {
    #[primary_key]
    #[unique]
    #[create_wrapper]
    key: String,
    
    /// The configuration value stored as an enum to support multiple types
    pub value: ConfigValue,
    
    /// Optional description of what this config does
    pub description: Option<String>,
    
    /// Scope: "database" for DB-wide settings, "user" for user-facing settings
    pub scope: ConfigScope,
    
    /// Who last modified this config
    pub last_modified_by: Option<Identity>,
    
    /// When this config was last updated
    pub last_modified_at: Timestamp,
}

/// Enum to support different configuration value types.
#[derive(Clone, Debug, PartialEq, spacetimedb::SpacetimeType)]
pub enum ConfigValue {
    /// String/text value
    Text(String),
    /// Signed 64-bit integer
    Integer(i64),
    /// Unsigned 64-bit integer
    UnsignedInteger(u64),
    /// 64-bit floating point number
    Float(f64),
    /// Boolean true/false
    Boolean(bool),
}

/// Scope of the configuration.
#[derive(Clone, Debug, PartialEq, spacetimedb::SpacetimeType)]
pub enum ConfigScope {
    /// Database-level settings (log levels, system limits)
    Database,
    /// User-facing settings (MOTD, multipliers)
    User,
}

// Common configuration key constants
pub const CONFIG_MOTD: &str = "motd";
pub const CONFIG_LOG_LEVEL: &str = "log_level";
pub const CONFIG_XP_MULTIPLIER: &str = "xp_multiplier";
pub const CONFIG_DAMAGE_MULTIPLIER: &str = "damage_multiplier";
pub const CONFIG_CHAT_MESSAGE_LIMIT: &str = "chat_message_limit";
pub const CONFIG_CHUNK_UPDATE_INTERVAL_MS: &str = "chunk_update_interval_ms";

// ============================================================================
// Permission Helpers
// ============================================================================

/// Checks if the sender has admin permissions (GameAdmin, ServerAdmin, or is the server itself).
///
/// # Arguments
/// * `ctx` - The reducer context
///
/// # Returns
/// true if the sender has admin permissions
fn has_config_admin_permission(ctx: &ReducerContext) -> bool {
    use crate::modules::roles::{has_role, RoleType};

    // Server itself always has permission
    if ctx.sender == ctx.identity() {
        return true;
    }

    // Check for admin roles
    has_role(ctx, &ctx.sender, &RoleType::GameAdmin)
        || has_role(ctx, &ctx.sender, &RoleType::ServerAdmin)
}

/// Requires admin permission, returning an error if not authorized.
///
/// # Arguments
/// * `ctx` - The reducer context
/// * `action` - Description of the action being attempted (for logging)
/// * `key` - The config key being accessed (for logging)
///
/// # Returns
/// Ok(()) if authorized, Err with message if not
fn require_config_admin_permission(
    ctx: &ReducerContext,
    action: &str,
    key: &str,
) -> Result<(), String> {
    if has_config_admin_permission(ctx) {
        spacetimedb::log::debug!(
            "ADMIN ACTION: User {} attempting to {} config '{}'",
            ctx.sender,
            action,
            key
        );
        Ok(())
    } else {
        spacetimedb::log::warn!(
            "SECURITY: User {} attempted to {} global config '{}' without proper permissions",
            ctx.sender,
            action,
            key
        );
        Err("Only GameAdmin, ServerAdmin, or server can modify global configuration".to_string())
    }
}

// ============================================================================
// Configuration Initialization
// ============================================================================

/// Initializes default global configurations.
/// Only runs if no configs exist yet.
///
/// # Arguments
/// * `ctx` - The reducer context
///
/// # Returns
/// Result indicating success or failure
pub fn init_default_configs(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx);
    
    // Check if configs already exist to avoid duplicates
    if dsl.get_all_global_configs().count() > 0 {
        spacetimedb::log::debug!("Global configs already initialized, skipping");
        return Ok(());
    }
    
    // Database-level configs
    dsl.create_global_config(CreateGlobalConfig {
        key: CONFIG_LOG_LEVEL.to_string(),
        value: ConfigValue::Text("INFO".to_string()),
        description: Some("Server logging level (ERROR, WARN, INFO, DEBUG, TRACE)".to_string()),
        scope: ConfigScope::Database,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;
    
    dsl.create_global_config(CreateGlobalConfig {
        key: CONFIG_CHAT_MESSAGE_LIMIT.to_string(),
        value: ConfigValue::UnsignedInteger(100),
        description: Some("Maximum number of global chat messages before archiving".to_string()),
        scope: ConfigScope::Database,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;
    
    // User-facing configs
    dsl.create_global_config(CreateGlobalConfig {
        key: CONFIG_MOTD.to_string(),
        value: ConfigValue::Text("Welcome to the game!".to_string()),
        description: Some("Message of the Day displayed to users".to_string()),
        scope: ConfigScope::User,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;
    
    dsl.create_global_config(CreateGlobalConfig {
        key: CONFIG_XP_MULTIPLIER.to_string(),
        value: ConfigValue::UnsignedInteger(1),
        description: Some("Experience points multiplier for all players".to_string()),
        scope: ConfigScope::User,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;
    
    dsl.create_global_config(CreateGlobalConfig {
        key: CONFIG_DAMAGE_MULTIPLIER.to_string(),
        value: ConfigValue::UnsignedInteger(1),
        description: Some("Damage multiplier for combat".to_string()),
        scope: ConfigScope::User,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;
    
    dsl.create_global_config(CreateGlobalConfig {
        key: CONFIG_CHUNK_UPDATE_INTERVAL_MS.to_string(),
        value: ConfigValue::UnsignedInteger(5000),
        description: Some("Interval in milliseconds between chunk position calculations".to_string()),
        scope: ConfigScope::Database,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;
    
    spacetimedb::log::debug!("Initialized default global configurations");
    Ok(())
}

/// Helper function to get a config value as text.
pub fn get_config_text(ctx: &ReducerContext, key: &str) -> Option<String> {
    let dsl = dsl(ctx);
    dsl.get_global_config_by_key(&key.to_string())
        .ok()
        .and_then(|config| match config.get_value() {
            ConfigValue::Text(s) => Some(s.clone()),
            _ => None,
        })
}

/// Helper function to get a config value as u64.
pub fn get_config_u64(ctx: &ReducerContext, key: &str) -> Option<u64> {
    let dsl = dsl(ctx);
    dsl.get_global_config_by_key(&key.to_string())
        .ok()
        .and_then(|config| match config.get_value() {
            ConfigValue::UnsignedInteger(n) => Some(*n),
            _ => None,
        })
}

/// Helper function to get a config value as i64.
pub fn get_config_i64(ctx: &ReducerContext, key: &str) -> Option<i64> {
    let dsl = dsl(ctx);
    dsl.get_global_config_by_key(&key.to_string())
        .ok()
        .and_then(|config| match config.get_value() {
            ConfigValue::Integer(n) => Some(*n),
            _ => None,
        })
}

/// Helper function to get a config value as f64.
pub fn get_config_f64(ctx: &ReducerContext, key: &str) -> Option<f64> {
    let dsl = dsl(ctx);
    dsl.get_global_config_by_key(&key.to_string())
        .ok()
        .and_then(|config| match config.get_value() {
            ConfigValue::Float(n) => Some(*n),
            _ => None,
        })
}

/// Helper function to get a config value as bool.
pub fn get_config_bool(ctx: &ReducerContext, key: &str) -> Option<bool> {
    let dsl = dsl(ctx);
    dsl.get_global_config_by_key(&key.to_string())
        .ok()
        .and_then(|config| match config.get_value() {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        })
}

/// Updates an existing configuration value.
/// Requires GameAdmin, ServerAdmin role, or server identity.
#[spacetimedb::reducer]
pub fn update_global_config(
    ctx: &ReducerContext,
    key: String,
    value: ConfigValue,
) -> Result<(), String> {
    require_config_admin_permission(ctx, "update", &key)?;

    let dsl = dsl(ctx);

    match dsl.get_global_config_by_key(&key) {
        Ok(mut config) => {
            let old_value = config.get_value().clone();
            config.set_value(value.clone());
            config.set_last_modified_by(Some(ctx.sender));
            config.set_last_modified_at(ctx.timestamp);

            dsl.update_global_config_by_key(config)?;
            spacetimedb::log::info!(
                "ADMIN ACTION: Updated config '{}' by {} (old: {:?}, new: {:?})",
                key,
                ctx.sender,
                old_value,
                value
            );
            Ok(())
        }
        Err(_) => {
            spacetimedb::log::warn!(
                "ADMIN ACTION: Failed - config key '{}' does not exist",
                key
            );
            Err(format!("Configuration key '{}' does not exist", key))
        }
    }
}

/// Creates a new configuration entry.
/// Requires GameAdmin, ServerAdmin role, or server identity.
#[spacetimedb::reducer]
pub fn create_global_config_entry(
    ctx: &ReducerContext,
    key: String,
    value: ConfigValue,
    description: Option<String>,
    scope: ConfigScope,
) -> Result<(), String> {
    require_config_admin_permission(ctx, "create", &key)?;

    let dsl = dsl(ctx);

    // Check if config already exists
    if dsl.get_global_config_by_key(&key).is_ok() {
        return Err(format!(
            "Configuration key '{}' already exists. Use update_global_config to modify it.",
            key
        ));
    }

    dsl.create_global_config(CreateGlobalConfig {
        key: key.clone(),
        value: value.clone(),
        description,
        scope,
        last_modified_by: Some(ctx.sender),
        last_modified_at: ctx.timestamp,
    })?;

    spacetimedb::log::info!(
        "ADMIN ACTION: Created config '{}' by {} with value: {:?}",
        key,
        ctx.sender,
        value
    );
    Ok(())
}

