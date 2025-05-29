use spacetimedb::{table, Identity, Timestamp};

#[table(name = player_status, public)]
pub struct PlayerStatus {
    #[primary_key]
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
