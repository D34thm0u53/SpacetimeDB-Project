use spacetimedb::{table, Identity, Timestamp, ReducerContext};
use spacetimedsl::dsl;

#[dsl(plural_name = player_statuses)]
#[table(name = player_status, public)]
pub struct PlayerStatus {
    #[primary_key]
    #[use_wrapper(path = crate::modules::player::PlayerAccountId)]
    id: u32,
    #[unique]
    pub identity: Identity, // Link to player by identity
    pub base_health: u32,   // 0-1000, typically 500
    pub shield: u32,        // 0-1000, typically 500
    pub concussed: f32,     // 0.0-1.0 (percentage, 1.0 = 100%)
    pub flashed: f32,       // 0.0-1.0
    pub emped: f32,         // 0.0-1.0
    pub poisoned: f32,      // 0.0-1.0
    pub last_updated: Timestamp,
}

impl PlayerStatus {
    pub fn total_health(&self) -> u32 {
        self.base_health + self.shield
    }
}

/// Applies damage from an attacker to a victim, updating shield and health accordingly.
/// Damage is absorbed by shield first, then by base_health. If both reach zero, the player is considered dead.
#[spacetimedb::reducer]
pub fn apply_damage(ctx: &ReducerContext, victim: crate::modules::player::PlayerAccountId, damage: u32) {
    log::info!("Applying {} damage to player {}", damage, victim);
    
    // Get DSL context
    let dsl = dsl(ctx);

    let mut status_record = match dsl.get_player_status_by_id(&victim) {
        Ok(record) => record,
        Err(e) => {
            log::warn!("Failed to find player status for player {}: {:?}", victim, e);
            return;
        }
    };

    // If already dead, do nothing
    if status_record.base_health == 0 {
        log::debug!("Player {} is already dead, ignoring damage", victim);
        return;
    }

    let original_health = status_record.base_health;
    let original_shield = status_record.shield;
    let mut remaining_damage = damage;

    // Apply to shield first
    if status_record.shield >= remaining_damage {
        status_record.shield -= remaining_damage;
        remaining_damage = 0;
        log::debug!("Shield absorbed {} damage for player {} (shield: {} -> {})", 
                   damage, victim, original_shield, status_record.shield);
    } else {
        remaining_damage -= status_record.shield;
        log::debug!("Shield depleted for player {} (absorbed {} damage, {} remaining)", 
                   victim, status_record.shield, remaining_damage);
        status_record.shield = 0;
    }

    // Apply any leftover damage to health
    if remaining_damage > 0 {
        if status_record.base_health >= remaining_damage {
            status_record.base_health -= remaining_damage;
            log::debug!("Health reduced for player {} (health: {} -> {})", 
                       victim, original_health, status_record.base_health);
        } else {
            log::warn!("Player {} died! (health: {} -> 0)", victim, original_health);
            status_record.base_health = 0;
        }
    }

    // Store values for logging before moving status_record
    let final_health = status_record.base_health;
    let final_shield = status_record.shield;

    // Update the PlayerStatus row
    if let Err(e) = dsl.update_player_status_by_id(status_record) {
        log::error!("Failed to update player status for player {}: {:?}", victim, e);
    } else {
        log::info!("Successfully applied {} damage to player {} (final health: {}, shield: {})", 
                  damage, victim, final_health, final_shield);
    }
}
