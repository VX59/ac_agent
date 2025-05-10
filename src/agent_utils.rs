use libc::close;
use rand::Rng;

use crate::err::Error;

use crate::hooks::{AC_FUNCTIONS, PROCESS};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
pub union WorldPos {
    pub v: Vec3,
    f: [f32; 3],
    i: [i32; 3],
}

impl Default for WorldPos {
    #[inline]
    fn default() -> Self {
        WorldPos { v: Vec3::default() }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct Traceresults {
    pub end: WorldPos,
    pub collided: bool,
    _padding: [u8; 3], // Ensure 4-byte alignment
}

#[repr(C)]
#[derive(Clone)]
pub struct Playerent {
    _pad_0x8: [u8; 0x8],
    pub head: Vec3,
    _pad_0x2c: [u8; 0x18],
    pub o: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    _pad_0x7a: [u8; 0x36],
    pub state: u8,
    _pad_0x100: [u8; 0x85],
    pub health: i32,
    _pad_0x158: [u8; 0x54],
    pub gun_wait: [i32; 9],
    _pad_0x320: [u8; 0x1a4],
    pub team: i32,
}

/// Used in navigation to scan for walls within the yaw range (phi_min, phi_max). Draws k rays in the bounded area
pub fn ray_scan(k: u32, yaw_radius: f32) -> Result<Vec<*const Traceresults>, Error> {
    unsafe {
        let mut rays: Vec<*const Traceresults> = vec![];

        let player1: &mut Playerent = match PROCESS.player1_ptr {
            Some(ptr) => &mut *ptr,
            None => return Err(Error::Player1Error),
        };

        let ray_magnitude: f32 = 100.0;
        let mut yaw_rng = rand::thread_rng();

        let theta_offset = 90.0; // to face forwards
        let min_yaw = player1.yaw - yaw_radius + theta_offset;
        let max_yaw = player1.yaw + yaw_radius + theta_offset;

        for _ in 0..k {
            let world_pos_from: WorldPos = WorldPos { v: player1.head };

            let random_yaw = yaw_rng.gen_range(min_yaw..max_yaw);

            let world_pos_to: WorldPos = WorldPos {
                v: Vec3 {
                    x: world_pos_from.v.x + f32::cos(random_yaw) * ray_magnitude,
                    y: world_pos_from.v.y + f32::sin(random_yaw) * ray_magnitude,
                    z: world_pos_from.v.z,
                },
            };

            let mut tr: Traceresults = Traceresults::default();

            match AC_FUNCTIONS.trace_line_func {
                Some(func) => func(world_pos_from, world_pos_to, 0, true, &mut tr),
                None => return Err(Error::Player1Error),
            };

            println!("TraceresultS end : {:?}", tr.end.v);
            println!("Collided : {:?}", tr.collided);

            rays.push(&tr);
        }

        Ok(rays)
    }
}

fn vec_distance(from: Vec3, to: Vec3) -> f32 {
    f32::sqrt(
        f32::powi(from.x - to.x, 2) + f32::powi(from.y - to.y, 2) + f32::powi(from.z - to.z, 2),
    )
}

fn is_trackable_target(player1: &Playerent, player: &Playerent) -> Result<bool, Error> {
    let mut trackable = false;
    if player.team != player1.team || player.state == 1 {
        trackable = true;
    }

    Ok(trackable)
}

/// Used in navigation to locate the closest enemy, even if they are not visible
fn closest_enemy(
    player1: &Playerent,
    players: *const *const u64,
) -> Result<Option<*const Playerent>, Error> {
    unsafe {
        let players_length: usize = {
            let length_addr = (players as u64 + 0xC) as *const u32;
            *length_addr as usize
        };

        if (*players).is_null() {
            return Err(Error::PlayersListError);
        }

        let players_list_ptr = *players;
        let players_list = std::slice::from_raw_parts(players_list_ptr.add(1), players_length - 1);

        let closest_enemy = players_list
            .iter()
            .map(|&ptr| {
                let player = &*(ptr as *const Playerent);

                (player, is_trackable_target(player1, player))
            })
            .filter_map(|(player, result)| match result {
                Ok(true) => Some(Ok((player, vec_distance(player1.head, player.head)))),
                Ok(false) => None,
                Err(e) => return Some(Err(e)),
            })
            .collect::<Result<Vec<(&Playerent, f32)>, Error>>()
            .map(|player_distances| {
                player_distances
                    .into_iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(player, _)| player)
            });

        match closest_enemy {
            Ok(Some(enemy)) => Ok(Some(enemy)),
            Ok(None) => Ok(None),
            Err(_) => Err(Error::PlayersListError),
        }
    }
}

/// used to locate nearest enemy after navigating a path
pub fn process_next_target() -> Result<(), Error> {
    unsafe {
        let player1: &mut Playerent = match PROCESS.player1_ptr {
            Some(ptr) => &mut *ptr,
            None => return Err(Error::Player1Error),
        };

        let players = match PROCESS.players_ptr {
            Some(ptr) => ptr,
            None => return Err(Error::PlayersListError),
        };

        if let Some(next_target) = closest_enemy(player1, players)? {
            // do some stuff here
        }

        Ok(())
    }
}
