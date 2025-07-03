use spacetimedb::{table, Identity, Timestamp, ReducerContext};
use spacetimedsl::dsl;

#[dsl(plural_name = player_statuses)]
#[table(name = player_status, public)]
pub struct PlayerStatus {
    #[primary_key]
    #[wrap]
    id: u64,
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
pub fn apply_damage(ctx: &ReducerContext, victim: Identity, damage: u32) {
    // Get DSL context
    let dsl = dsl(ctx);

    // Fetch the victim's PlayerStatus
    let mut status = match dsl.get_player_status_by_identity(&victim) {
        Some(s) => s,
        None => {
            // Victim does not exist
            return;
        }
    };

        // If already dead, do nothing
    if status.base_health == 0 && status.shield == 0 {
        return;
    }

    let mut remaining_damage = damage;

    
    // Apply to shield first
    if status.shield >= remaining_damage {
        status.shield -= remaining_damage;
        remaining_damage = 0;
    } else {
        remaining_damage -= status.shield;
        status.shield = 0;
    }


    // Apply any leftover damage to health
    if remaining_damage > 0 {
        if status.base_health >= remaining_damage {
            status.base_health -= remaining_damage;
        } else {
            status.base_health = 0;
        }
    }

    // Update the PlayerStatus row
    if let Err(e) = dsl.update_player_status_by_identity(status) {
        log::error!("Failed to update player status: {:?}", e);
    }

}
