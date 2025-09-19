use spacetimedb::{table, Timestamp};
use spacetimedsl::dsl;

/// Table for weapon definitions and stats.
#[dsl(plural_name = weapons)]
#[table(name = weapon, public)]
pub struct Weapon {
    #[primary_key]
    #[auto_inc]
    #[wrap]    
    id: u32, // Unique weapon model identifier
    pub name: String, // Human-readable name
    pub description: String, // Description for UI
    pub damage: u16, // Base damage per shot
    pub rate_of_fire: u16, // Rounds per minute (RPM)
    pub damage_type: u16, // damage type value (0-1000)
    pub magazine_size: u16, // Number of rounds per magazine (0-1000)
    pub reload_time: u16, // Reload time in tenths of a second (e.g. 15 = 1.5s)
    pub projectile_speed: u16, // Meters per second (0-1000)
    pub range: u16, // Effective range in meters (0-1000)
    pub headshot_multiplier: u16, // Multiplier x100 (e.g. 150 = 1.5x)
    // --- Advanced fields ---
    pub equip_time_ms: u16, // Time to equip (ms)
    pub unequip_time_ms: u16, // Time to unequip (ms)
    pub to_ironsights_ms: u16, // Time to enter iron sights (ms)
    pub from_ironsights_ms: u16, // Time to exit iron sights (ms)
    pub sprint_recovery_ms: u16, // Time to recover from sprint (ms)
    pub move_modifier: u16, // Move speed modifier x100 (e.g. 75 = 0.75)
    pub zoom: u16, // Zoom x100 (e.g. 135 = 1.35)
    pub recoil_first_shot: u16, // First shot recoil x100 (e.g. 210 = 2.1)
    pub recoil_horizontal_min: u16, // Horizontal recoil min x1000 (e.g. 243 = 0.243)
    pub recoil_horizontal_max: u16, // Horizontal recoil max x1000 (e.g. 273 = 0.273)
    pub recoil_horizontal_tolerance: u16, // Horizontal tolerance x1000 (e.g. 735 = 0.735)
    pub recoil_magnitude_min: u16, // Recoil magnitude min x100 (e.g. 31 = 0.31)
    pub recoil_magnitude_max: u16, // Recoil magnitude max x100 (e.g. 31 = 0.31)
    pub recoil_recovery_rate: u16, // Recoil recovery rate
    pub recoil_recovery_delay_ms: u16, // Recoil recovery delay (ms)
    pub recoil_recovery_accel: u16, // Recoil recovery acceleration
    pub cof_min: u16, // Cone of fire min x100 (e.g. 125 = 1.25)
    pub cof_max: u16, // Cone of fire max x100 (e.g. 700 = 7.0)
    pub cof_grow_rate: u16, // Cone of fire grow rate
    pub cof_recovery_rate: u16, // Cone of fire recovery rate
    pub cof_recovery_delay_ms: u16, // Cone of fire recovery delay (ms)
    pub gravity: u16, // Gravity x100 (e.g. 1125 = 11.25)
    pub projectile_lifespan_ms: u16, // Projectile lifespan (ms)
    pub can_iron_sight: u8, // 1 if can iron sight, 0 otherwise
    pub reload_ammo_fill_ms: u16, // Time to fill ammo during reload (ms)
    pub reload_chamber_ms: u16, // Time to chamber during reload (ms)
    pub fire_refire_ms: u16, // Time between shots (ms)
    pub automatic: u8, // 1 if automatic, 0 if semi-automatic
    created_at: Timestamp,
    modified_at: Timestamp,
}


//// Table for tracking which items (weapons, grenades, etc.) a player has access to.
#[dsl(plural_name = player_items)]
#[table(name = player_item, public, index(name = idx_player_item, btree(columns = [player_id, item_id])))]
pub struct PlayerItem {
    #[primary_key]
    #[auto_inc]
    #[wrap]    
    id: u32, // Surrogate PK (auto-increment)
    pub player_id: spacetimedb::Identity, // Player's unique identity
    pub item_id: u32, // Item model id (weapon, grenade, etc.)
    pub unlocked_at: Timestamp, // When the item was unlocked for the player
    pub expires_at: Option<Timestamp>, // Optional: when the item expires (for rentals, timed unlocks)
}


impl Weapon {
    /// Returns true if the weapon is hitscan (projectile_speed == 0)
    pub fn is_hitscan(&self) -> bool {
        self.projectile_speed == 0
    }
}



/// Inserts a set of default weapons into the Weapon table. Intended to be called on database initialization.
pub fn initialize_default_weapons(ctx: &spacetimedb::ReducerContext) {
    let dsl = dsl(ctx);
    // 750 RPM @ 143 damage



/* Carbines

*/

    let _ = dsl.create_weapon(
        &"LC3 Jaguar".to_string(),
        &"LC3 Jaguar: Boasting a longer barrel than its compact Light Carbine cousin, the mobile LC3 Jaguar was designed to provide accurate hip fire at longer distances than the Lynx. TR use only.".to_string(),
        143, // damage (max_damage)
        750, // rate_of_fire (calculated from fire_refire_ms: 80ms -> 750 RPM)
        0,   // penetration (armor_penetration)
        40,  // magazine_size (not in JSON, use Carbine's value for now)
        28,  // reload_time (reload_time_ms: 2755ms -> 27.5, round up to 28 tenths)
        440, // projectile_speed (projectile_speed_override)
        60,  // range (min_damage_range)
        135, // headshot_multiplier (damage_head_multiplier: 1.35x, but JSON says 1, so 100)
        550, // equip_time_ms
        250, // unequip_time_ms
        150, // to_ironsights_ms
        150, // from_ironsights_ms
        300, // sprint_recovery_ms
        75,  // move_modifier (0.75)
        135, // zoom (zoom_default: 1.35)
        210, // recoil_first_shot (2.1)
        243, // recoil_horizontal_min (0.243)
        273, // recoil_horizontal_max (0.273)
        735, // recoil_horizontal_tolerance (0.735)
        31,  // recoil_magnitude_min (0.31)
        31,  // recoil_magnitude_max (0.31)
        18,  // recoil_recovery_rate
        80,  // recoil_recovery_delay_ms
        1000, // recoil_recovery_accel
        10,  // cof_min (ADS cof_min: 0.1 * 100)
        300, // cof_max (ADS cof_max: 3.0 * 100)
        50,  // cof_grow_rate
        20,  // cof_recovery_rate
        0,   // cof_recovery_delay_ms
        1125, // gravity (11.25 * 100)
        1500, // projectile_lifespan_ms (1.5s)
        1,   // can_iron_sight
        3090, // reload_ammo_fill_ms
        1135, // reload_chamber_ms
        80,   // fire_refire_ms
        1,    // automatic
    );
/* LMGs

*/

    let _ = dsl.create_weapon(
        &"NC6 Gauss SAW".to_string(),
        &"NC6 Gauss SAW: The NC6 Gauss SAW's low production cost, reliability, and sheer stopping power lead to wide-spread adoption of the weapon in the Auraxian War's early years. NC use only.".to_string(),
        200, // damage (typical for Gauss SAW, not in JSON, but 200 is standard)
        500, // rate_of_fire (RPM, not in JSON, typical for Gauss SAW)
        0,   // penetration (armor_penetration)
        100, // magazine_size (not in JSON, typical for Gauss SAW)
        60,  // reload_time (not in JSON, 6.0s -> 60 tenths)
        600, // projectile_speed (not in JSON, typical for Gauss SAW)
        85,  // range (not in JSON, use max_view_pitch as a placeholder)
        150, // headshot_multiplier (1.5x)
        1200, // equip_time_ms
        250,  // unequip_time_ms
        150,  // to_ironsights_ms
        150,  // from_ironsights_ms
        300,  // sprint_recovery_ms
        100,  // move_modifier (1.0)
        100,  // zoom (1.0)
        250,  // recoil_first_shot (not in JSON, typical for LMG)
        300,  // recoil_horizontal_min (not in JSON, typical for LMG)
        350,  // recoil_horizontal_max (not in JSON, typical for LMG)
        800,  // recoil_horizontal_tolerance (not in JSON, typical for LMG)
        40,   // recoil_magnitude_min (not in JSON, typical for LMG)
        40,   // recoil_magnitude_max (not in JSON, typical for LMG)
        15,   // recoil_recovery_rate (not in JSON, typical for LMG)
        100,  // recoil_recovery_delay_ms (not in JSON, typical for LMG)
        1000, // recoil_recovery_accel (not in JSON, typical for LMG)
        20,   // cof_min (not in JSON, typical for LMG)
        600,  // cof_max (not in JSON, typical for LMG)
        60,   // cof_grow_rate (not in JSON, typical for LMG)
        18,   // cof_recovery_rate (not in JSON, typical for LMG)
        0,    // cof_recovery_delay_ms
        1125, // gravity (11.25 * 100)
        2000, // projectile_lifespan_ms (not in JSON, typical for LMG)
        1,    // can_iron_sight
        4000, // reload_ammo_fill_ms (not in JSON, typical for LMG)
        2000, // reload_chamber_ms (not in JSON, typical for LMG)
        120,  // fire_refire_ms (not in JSON, typical for LMG)
        1,    // automatic
    );
/* Sniper Rifles

*/

    let _ = dsl.create_weapon(
        &"TSAR-42".to_string(),
        &"TSAR-42: Deadly in the hands of a skilled Infiltrator, the bolt action TSAR-42's features a unique bolt rotation system designed to improve the time between shots. TR use only.".to_string(),
        700, // damage (typical for bolt-action sniper, not in JSON, use 700)
        50,  // rate_of_fire (RPM, not in JSON, typical for bolt-action)
        0,   // penetration (armor_penetration)
        5,   // magazine_size (not in JSON, typical for TSAR-42)
        30,  // reload_time (not in JSON, 3.0s -> 30 tenths)
        0,   // projectile_speed (not in JSON, 0 for hitscan or use 850 if projectile)
        300, // range (not in JSON, use 300 as a long-range default)
        200, // headshot_multiplier (2.0x)
        850, // equip_time_ms
        250, // unequip_time_ms
        200, // to_ironsights_ms
        150, // from_ironsights_ms
        300, // sprint_recovery_ms
        100, // move_modifier (1.0)
        100, // zoom (1.0)
        400, // recoil_first_shot (not in JSON, typical for bolt-action)
        0,   // recoil_horizontal_min
        0,   // recoil_horizontal_max
        0,   // recoil_horizontal_tolerance
        50,  // recoil_magnitude_min
        50,  // recoil_magnitude_max
        10,  // recoil_recovery_rate
        100, // recoil_recovery_delay_ms
        500, // recoil_recovery_accel
        50,  // cof_min
        200, // cof_max
        10,  // cof_grow_rate
        10,  // cof_recovery_rate
        0,   // cof_recovery_delay_ms
        1000, // gravity (10.0 * 100)
        2000, // projectile_lifespan_ms (not in JSON, typical for sniper)
        1,    // can_iron_sight
        4000, // reload_ammo_fill_ms (not in JSON, typical for sniper)
        2000, // reload_chamber_ms (not in JSON, typical for sniper)
        1200, // fire_refire_ms (not in JSON, typical for bolt-action)
        0,    // automatic
    );
}


