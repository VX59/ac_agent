use crate::err::Error;
use log::debug;

use crate::hooks::{AC_FUNCTIONS, PLAYER1};

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
    pub pointer: Option<&'static ()>,
    _pad_0x2c: [u8; 0x24],
    pub o: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    _pad_0x100: [u8; 0xbc],
    pub health: i32,
    _pad_0x320: [u8; 0x21c],
    pub team: i32,
}

/// Used in navigation to scan for walls within the yaw range (phi_min, phi_max). Draws k rays in the bounded area
pub fn ray_scan(k: u32, phi_min: f32, phi_max: f32) -> Result<Vec<*const Traceresults>, Error> {
    let mut rays: Vec<*const Traceresults> = vec![];

    let player1 = match unsafe { PLAYER1 } {
        Some(addr) => {
            let addr = addr as *const *const Playerent;
            unsafe { &**addr }
        }
        None => return Err(Error::Player1Error),
    };

    println!(
        "player1 pos {} {} {}",
        player1.o.x, player1.o.y, player1.o.z
    );

    for _ in 0..k {
        let from: Vec3 = Vec3 {
            x: player1.o.x,
            y: player1.o.y,
            z: 5.0, // the head pos
        };

        let world_pos_from: WorldPos = WorldPos { v: from };

        let ray_magnitude: f32 = 100.0;

        // add random yaw here
        let to: Vec3 = Vec3 {
            x: from.x + f32::cos(player1.yaw) * ray_magnitude,
            y: from.y + f32::sin(player1.yaw) * ray_magnitude,
            z: from.z,
        };

        let world_pos_to: WorldPos = WorldPos { v: to };

        let mut tr: Traceresults = Traceresults::default();

        unsafe {
            match AC_FUNCTIONS.trace_line_func {
                Some(func) => func(world_pos_from, world_pos_to, 0, true, &mut tr),
                None => return Err(Error::Player1Error),
            };

            println!("TraceresultS end : {:?}", tr.end.v);
            println!("Collided : {:?}", tr.collided);
        }

        rays.push(&tr);
    }

    Ok(rays)
}

pub fn is_enemy_visible(player1: &Playerent, player: &Playerent) -> Result<bool, Error> {
    let from: Vec3 = Vec3 {
        x: player1.o.x,
        y: player1.o.y,
        z: player1.o.z + 5.0,
    };

    let world_pos_from: WorldPos = WorldPos { v: from };

    let to: Vec3 = Vec3 {
        x: player.o.x,
        y: player.o.y,
        z: player.o.z + 5.0,
    };

    let world_pos_to: WorldPos = WorldPos { v: to };

    unsafe {
        match AC_FUNCTIONS.is_visible_func {
            Some(func) => return Ok(func(world_pos_from, world_pos_to, 0, false)),
            None => return Err(Error::TraceLineError),
        };
    }
}

/// Used in navigation to locate the closest enemy, even if they are not visible
pub fn closest_enemy(
    players_list_ptr: *const u64,
    players_length: usize,
    player1: &Playerent,
) -> Result<&Playerent, Error> {
    let from: Vec3 = Vec3 {
        x: player1.o.x,
        y: player1.o.y,
        z: player1.o.z,
    };

    if players_list_ptr.is_null() {
        return Err(Error::PlayersListError);
    }

    let mut min_dist = f32::MAX;
    let mut closest_enemy: Option<&Playerent> = None; // player1 is 0

    for i in 0..players_length {
        let addr = unsafe { *players_list_ptr.offset(i as isize) } as *const Playerent;
        let player: &Playerent = unsafe { &*addr };

        if player.team == player1.team {
            continue;
        }

        let to: Vec3 = Vec3 {
            x: player.o.x,
            y: player.o.y,
            z: player.o.z,
        };
        let distance = f32::sqrt(
            f32::powi(from.x - to.x, 2) + f32::powi(from.y - to.y, 2) + f32::powi(from.z - to.z, 2),
        );

        if distance < min_dist {
            min_dist = distance;
            closest_enemy = Some(player);
        }
    }

    match closest_enemy {
        Some(enemy) => Ok(enemy),
        None => Err(Error::PlayersListError),
    }
}
