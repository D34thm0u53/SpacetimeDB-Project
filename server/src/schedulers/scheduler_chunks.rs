use std::{time::Duration};


use spacetimedb::{ReducerContext};
use spacetimedsl::{dsl};

use crate::modules::{player::*, entity_positions::*, common::*};

#[dsl(plural_name = chunk_check_timers)]
#[spacetimedb::table(name = chunk_check_timer, scheduled(calculate_current_chunks))]
pub struct ChunkCheckTimer {
    #[primary_key]
    #[auto_inc]
    #[wrap]
    pub scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
    current_update: u8,
}

pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx); // Waiting for DSL implementation of timers

    dsl.create_chunk_check_timer(spacetimedb::ScheduleAt::Interval(Duration::from_millis(5000).into()), 0)?;
    Ok(())
}

#[spacetimedb::reducer]
pub fn calculate_current_chunks(ctx: &ReducerContext, _timer: ChunkCheckTimer) -> Result<(), String> {
    let dsl = dsl(ctx);
    try_server_only(ctx)?;

    for player in dsl.get_all_players() {
        if let Some(mut pos) = dsl.get_entity_position_by_player_identity(&player.identity) {
            let chunk_x: i32 = (pos.x / 50.0).floor() as i32;
            let chunk_z: i32 = (pos.z / 50.0).floor() as i32;
            if pos.chunk_x != chunk_x || pos.chunk_z != chunk_z {
                log::info!("Updating player {} chunk to ({}, {})", player.identity, chunk_x, chunk_z);
                pos.chunk_x = chunk_x;
                pos.chunk_z = chunk_z;
                dsl.update_entity_position_by_player_identity(pos).ok();
            }
        }
    }
    Ok(())
}
