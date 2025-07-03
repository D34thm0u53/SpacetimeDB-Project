use std::{time::Duration};


use spacetimedb::{ReducerContext};
use spacetimedsl::{dsl};

use crate::modules::{player::*, entity_position::*};


/* 
Tables
- global_configuration: Stores global configuration settings for the server.
- chunk_check_timer: A scheduled task that checks and updates player chunk positions based on their current entity positions.
*/

#[dsl(plural_name = global_configurations)]
#[spacetimedb::table(name = global_configuration)]
pub struct GlobalConfiguration {
    #[primary_key]
    #[auto_inc]
    #[wrap]    
    id: u64,
    pub scheduled_id: u64,
    pub current_update: u64,
}

#[dsl(plural_name = chunk_check_timers)]
#[spacetimedb::table(name = chunk_check_timer, scheduled(calculate_current_chunks))]
pub struct ChunkCheckTimer {
    #[primary_key]
    #[auto_inc]
    #[wrap]    
    id: u64,
    pub scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
    current_update: u8,
}


/* 
Reducers
- init: Initializes the global configuration and sets up the chunk check timer.
- calculate_current_chunks: A scheduled task that updates player chunk positions based on their current entity positions.
*/

pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = dsl(ctx); // Waiting for DSL implementation of timers

    dsl.create_chunk_check_timer(spacetimedb::ScheduleAt::Interval(Duration::from_millis(5000).into()), 0)?;
    Ok(())
}

#[spacetimedb::reducer]
pub fn calculate_current_chunks(ctx: &ReducerContext, mut timer: ChunkCheckTimer) -> Result<(), String> {
    let dsl = dsl(ctx);
    for player in dsl.get_all_online_players() {

        timer.current_update += 1;
        let _ = dsl.update_chunk_check_timer_by_scheduled_id(timer.clone());
    
    

        // if the Online player has a position record
        if let Some(ent_pos) = dsl.get_entity_position_by_player_identity(&player.identity) {
            // if the player has a chunk record, update it
            if let Some(mut ent_chunk) = dsl.get_entity_chunk_by_player_identity(&player.identity) {
                let chunk_x: i32 = (ent_pos.x / 50.0).floor() as i32;
                let chunk_z: i32 = (ent_pos.z / 50.0).floor() as i32;
                if ent_chunk.chunk_x != chunk_x || ent_chunk.chunk_z != chunk_z {
                    log::info!("Updating player {} chunk to ({}, {})", player.identity, chunk_x, chunk_z);
                    ent_chunk.chunk_x = chunk_x;
                    ent_chunk.chunk_z = chunk_z;
                    dsl.update_entity_position_by_player_identity(ent_pos).ok();
                }
            }
            // if the player does not have a chunk record, create it
            else {
                let chunk_x: i32 = (ent_pos.x / 50.0).floor() as i32;
                let chunk_z: i32 = (ent_pos.z / 50.0).floor() as i32;
                log::info!("Creating player {} chunk at ({}, {})", player.identity, chunk_x, chunk_z);
                dsl.create_entity_chunk(player.identity, chunk_x, chunk_z, 0, 0, 0, 0)?;
            }
        }
        else {
            dsl.create_entity_position(player.identity, 0.0, 0.0, 0.0).expect("Failed to create entity position for player");
        }
        

    }
    Ok(())
}
