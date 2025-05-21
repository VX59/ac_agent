use std::f32::consts::PI;

use crate::agent_utils::{Playerent, Vec3, WorldPos};
use crate::err::Error;
use crate::esp::{draw_player_box, draw_player_healthbar, draw_player_traceline};
use crate::hooks::{AC_FUNCTIONS, PROCESS};

pub fn is_valid_target(player1: &Playerent, player: &Playerent) -> Result<bool, Error> {
    if player.state == 1 || player1.team == player.team {
        return Ok(false);
    }

    unsafe {
        match AC_FUNCTIONS.is_visible_func {
            Some(func) => {
                let result = func(
                    WorldPos { v: player1.head },
                    WorldPos { v: player.head },
                    0,
                    false,
                );
                return Ok(result);
            }
            None => return Err(Error::TraceLineError),
        };
    }
}

fn viewangle(player1: &Playerent, player: &Playerent) -> Vec3 {
    let dx = player.o.x - player1.o.x;
    let dy = player.o.y - player1.o.y;

    let yaw = (dy.atan2(dx) + (PI / 2.0)) * (180.0 / PI);

    let dz = player.head.z - player1.head.z;
    let h: f32 = (dx.powi(2) + dy.powi(2)).sqrt();

    let pitch = dz.atan2(h) * (180.0 / PI);

    let hr = (pitch.powi(2) + yaw.powi(2)).sqrt();

    return Vec3 {
        x: yaw,
        y: pitch,
        z: hr,
    };
}

pub fn is_combat_ready(player1: &Playerent) -> Result<bool, Error> {
    // firing state is different for each gun (most likely related to damage) & invalid firing states are just some # bigger than that
    let gun_state_thresholds = [160, 720, 880, 80, 1500, 120];

    unsafe {
        let gun_wait_list = std::slice::from_raw_parts(&(player1.gun_wait[3]), 6);
        if gun_wait_list
            .iter()
            .zip(gun_state_thresholds.iter())
            .any(|(&gun_state, &threshold)| gun_state > threshold)
        {
            return Ok(false);
        }
    }

    if player1.state == 1 {
        return Ok(false);
    }
    Ok(true)
}

pub fn get_best_viewangles(
    player1: &Playerent,
    players: *const *const u64,
) -> Result<Option<(Vec3, &Playerent)>, Error> {
    unsafe {
        let players_length: usize = {
            let length_addr = (players as u64 + 0xC) as *const u32;
            *length_addr as usize
        };

        let players_list_ptr = *players;

        let players_list = std::slice::from_raw_parts(players_list_ptr.add(1), players_length - 1);

        match is_combat_ready(player1) {
            Ok(result) => {
                if !result {
                    return Ok(None);
                }
            }
            Err(_) => return Err(Error::Player1Error),
        };

        let min_view_angle = players_list
            .iter()
            .map(|&ptr| {
                let player = &*(ptr as *const Playerent);
                (player, is_valid_target(player1, player))
            })
            .filter_map(|(player, result)| match result {
                Ok(true) => Some(Ok((viewangle(player1, player), player))),
                Ok(false) => None,
                Err(e) => return Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|vec| {
                vec.into_iter().min_by(|a, b| {
                    a.0.z
                        .partial_cmp(&b.0.z)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });

        match min_view_angle {
            Ok(Some(result)) => Ok(Some(result)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub fn update_agent_viewangles() -> Result<(), Error> {
    unsafe {
        let player1: &mut Playerent = match PROCESS.player1_ptr {
            Some(ptr) => &mut *ptr,
            None => return Err(Error::Player1Error),
        };

        let players = match PROCESS.players_ptr {
            Some(ptr) => ptr,
            None => return Err(Error::PlayersListError),
        };

        if (*players).is_null() {
            return Err(Error::PlayersListError);
        }

        if let Some((view_angle, player)) = get_best_viewangles(player1, players)? {
            let color = Vec3 {
                x: 255.0,
                y: 0.0,
                z: 255.0,
            };
            draw_player_box(player, color);
            draw_player_healthbar(player);
            draw_player_traceline(player, color);
            player1.yaw = view_angle.x;
            player1.pitch = view_angle.y;
        }

        Ok(())
    }
}
