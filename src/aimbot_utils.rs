use std::f32::consts::PI;

use crate::agent_utils::{Playerent, Vec3, WorldPos};
use crate::err::{self, Error};
use crate::hooks::{AC_FUNCTIONS, PROCESS};

pub fn is_valid_target(player1: &Playerent, player: &Playerent) -> Result<bool, Error> {
    let from: Vec3 = Vec3 {
        x: player1.o.x,
        y: player1.o.y,
        z: player1.o.z + 5.0,
    };

    let world_pos_from: WorldPos = WorldPos { v: from };

    let to: Vec3 = Vec3 {
        x: player.o.x,
        y: player.o.y,
        z: player.o.z + 5.0, // neck
    };

    let world_pos_to: WorldPos = WorldPos { v: to };

    if player.state == 1 {
        return Ok(false);
    }

    if player1.team == player.team {
        return Ok(false);
    }

    unsafe {
        match AC_FUNCTIONS.is_visible_func {
            Some(func) => return Ok(func(world_pos_from, world_pos_to, 0, false)),
            None => return Err(Error::TraceLineError),
        };
    }
}

fn viewangle(player1: &Playerent, player: &Playerent) -> Vec3 {
    let dx = player.o.x - player1.o.x;
    let dy = player.o.y - player1.o.y;
    let dz = player.o.z - player1.o.z;

    let h: f32 = (dx.powf(2.0) + dy.powf(2.0)).sqrt();

    let mut yaw = (dy / dx).atan() + (PI / 2.0);
    let mut pitch = (dz / h).atan();

    // convert radians to degrees

    yaw = yaw * (180.0 / PI);
    pitch = pitch * (180.0 / PI);

    let hr = (pitch.powi(2) + yaw.powi(2)).sqrt();

    let view_angle = Vec3 {
        x: yaw,
        y: pitch,
        z: hr,
    };

    return view_angle;
}

pub fn get_best_viewangles() -> Result<Option<Vec3>, Error> {
    unsafe {
        let players_ptr = match PROCESS.players_addr {
            Some(addr) => {
                let addr = addr as *const *const u64;
                addr
            }
            None => return Err(Error::PlayersListError),
        };

        let player1 = match PROCESS.player1_addr {
            Some(addr) => {
                let addr = addr as *const *const Playerent;
                &**addr
            }
            None => return Err(Error::Player1Error),
        };

        if (*players_ptr).is_null() {
            return Err(Error::PlayersListError);
        }

        let players_length: u32 = {
            let length_addr = (players_ptr as u64 + 0xC) as *const u32;
            *length_addr as u32
        };

        let mut min_view_angle_mag = f32::MAX;
        let mut min_view_angle: Option<Vec3> = None;

        let players_list_ptr = *players_ptr;

        for i in 1..players_length {
            let addr = *players_list_ptr.offset(i as isize) as *const Playerent;
            let player: &Playerent = &*addr;

            match is_valid_target(player1, player) {
                Ok(result) => {
                    if result {
                        let view_angle = viewangle(player1, player);

                        if view_angle.z < min_view_angle_mag {
                            min_view_angle_mag = view_angle.z;
                            min_view_angle = Some(view_angle);
                        }
                    }
                }
                Err(_) => return Err(Error::PlayersListError),
            }
        }
        Ok(min_view_angle)
    }
}

pub fn update_agent_viewangles() -> Result<(), Error> {
    unsafe {
        let player1 = match PROCESS.player1_addr {
            Some(addr) => {
                let addr = addr as *const *mut Playerent;
                *addr
            }
            None => return Err(Error::Player1Error),
        };
        match get_best_viewangles()? {
            Some(view_angle) => {
                (*player1).yaw = view_angle.x;
                (*player1).pitch = view_angle.y;
            }
            None => {}
        };
        Ok(())
    }
}
