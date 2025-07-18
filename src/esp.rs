use crate::{
    agent_utils::{PersistentEnt, Playerent, Vec3, WorldPos},
    err::Error,
    hooks::{AC_FUNCTIONS, OPENGL_FUNCTIONS, PROCESS},
};

pub struct GlModes {
    gl_blend: u32,
    gl_texture_2d: u32,
    gl_line_loop: i32,
    gl_depth_test: u32,
    gl_cull_face: u32,
}

static GLMODES: GlModes = GlModes {
    gl_blend: 0x0BE2,
    gl_texture_2d: 0x0DE1,
    gl_line_loop: 0x0002,
    gl_depth_test: 0x0B71,
    gl_cull_face: 0x0B44,
};

/// uses mvpmatrix to project 3d coordinates into 2d space on the screen
pub fn transform(pos: Vec3) -> Result<Vec3, Error> {
    unsafe {
        if let Some(mvp_ptr) = PROCESS.mvpmatrix_ptr {
            let mvp = std::slice::from_raw_parts(mvp_ptr, 16);

            let xclip = mvp[0] * pos.x + mvp[4] * pos.y + mvp[8] * pos.z + mvp[12];
            let yclip = mvp[1] * pos.x + mvp[5] * pos.y + mvp[9] * pos.z + mvp[13];
            let wclip = mvp[3] * pos.x + mvp[7] * pos.y + mvp[11] * pos.z + mvp[15];

            let ndc_x = xclip / wclip;
            let ndc_y = yclip / wclip;

            if wclip < 0.1 {
                return Err(Error::BehindCamera);
            }

            if let (Some(screenw), Some(screenh)) = (PROCESS.screenw_ptr, PROCESS.screenh_ptr) {
                let screen_width = *screenw as f32;
                let screen_height = *screenh as f32;

                let screen_x = (screen_width / 2.0) * (1.0 + ndc_x);
                let screen_y = (screen_height / 2.0) * (1.0 - ndc_y);

                Ok(Vec3 {
                    x: screen_x,
                    y: screen_y,
                    z: 0.0,
                })
            } else {
                return Err(Error::SymbolError);
            }
        } else {
            return Err(Error::SymbolError);
        }
    }
}

pub fn prepare_to_draw() {
    unsafe {
        if let Some(gl_disable) = OPENGL_FUNCTIONS.gl_disable {
            gl_disable(GLMODES.gl_blend);
            gl_disable(GLMODES.gl_texture_2d);
            gl_disable(GLMODES.gl_depth_test);
            gl_disable(GLMODES.gl_cull_face);
        }
        if let Some(gl_matrix_mode) = OPENGL_FUNCTIONS.gl_matrix_mode {
            gl_matrix_mode(0x1701); // GL_PROJECTION
        }

        if let Some(gl_push_matrix) = OPENGL_FUNCTIONS.gl_push_matrix {
            gl_push_matrix();
        }

        if let Some(gl_load_identity) = OPENGL_FUNCTIONS.gl_load_identity {
            gl_load_identity();
        }

        if let (Some(gl_ortho), Some(screenw), Some(screenh)) = (
            OPENGL_FUNCTIONS.gl_ortho,
            PROCESS.screenw_ptr,
            PROCESS.screenh_ptr,
        ) {
            let width = *screenw;
            let height = *screenh;
            gl_ortho(0.0, width as f64, height as f64, 0.0, -1.0, 1.0);
        }

        if let Some(gl_matrix_mode) = OPENGL_FUNCTIONS.gl_matrix_mode {
            gl_matrix_mode(0x1700); // GL_MODELVIEW
        }

        if let Some(gl_push_matrix) = OPENGL_FUNCTIONS.gl_push_matrix {
            gl_push_matrix();
        }

        if let Some(gl_load_identity) = OPENGL_FUNCTIONS.gl_load_identity {
            gl_load_identity();
        }
    }
}

pub fn cleanup_draw() {
    unsafe {
        if let Some(gl_enable) = OPENGL_FUNCTIONS.gl_enable {
            gl_enable(GLMODES.gl_blend);
            gl_enable(GLMODES.gl_texture_2d);
            gl_enable(GLMODES.gl_depth_test);
            gl_enable(GLMODES.gl_cull_face);
        }
        if let Some(gl_matrix_mode) = OPENGL_FUNCTIONS.gl_matrix_mode {
            gl_matrix_mode(0x1700); // GL_MODELVIEW
        }
        if let Some(gl_pop_matrix) = OPENGL_FUNCTIONS.gl_pop_matrix {
            gl_pop_matrix();
        }

        if let Some(gl_matrix_mode) = OPENGL_FUNCTIONS.gl_matrix_mode {
            gl_matrix_mode(0x1701); // GL_PROJECTION
        }
        if let Some(gl_pop_matrix) = OPENGL_FUNCTIONS.gl_pop_matrix {
            gl_pop_matrix();
        }
    }
}

pub fn draw_rectangle(
    top_left_x: f32,
    top_left_y: f32,
    bottom_right_x: f32,
    bottom_right_y: f32,
    color: Vec3,
) {
    prepare_to_draw();

    unsafe {
        if let Some(gl_color_3f) = OPENGL_FUNCTIONS.gl_color_3f {
            gl_color_3f(color.x, color.y, color.z);
        }

        if let Some(gl_line_width) = OPENGL_FUNCTIONS.gl_line_width {
            gl_line_width(2.0);
        }

        if let Some(gl_begin) = OPENGL_FUNCTIONS.gl_begin {
            gl_begin(GLMODES.gl_line_loop);
        }

        if let Some(gl_vertex_2f) = OPENGL_FUNCTIONS.gl_vertex_2f {
            gl_vertex_2f(top_left_x, top_left_y);
            gl_vertex_2f(bottom_right_x, top_left_y);
            gl_vertex_2f(bottom_right_x, bottom_right_y);
            gl_vertex_2f(top_left_x, bottom_right_y);
        }

        if let Some(gl_end) = OPENGL_FUNCTIONS.gl_end {
            gl_end();
        }
    }
    cleanup_draw();
}

pub fn draw_line(x_a: f32, y_a: f32, x_b: f32, y_b: f32, width: f32, color: Vec3) {
    prepare_to_draw();
    unsafe {
        if let Some(gl_color_3f) = OPENGL_FUNCTIONS.gl_color_3f {
            gl_color_3f(color.x, color.y, color.z);
        }

        if let Some(gl_line_width) = OPENGL_FUNCTIONS.gl_line_width {
            gl_line_width(width);
        }

        if let Some(gl_begin) = OPENGL_FUNCTIONS.gl_begin {
            gl_begin(GLMODES.gl_line_loop);
        }

        if let Some(gl_vertex_2f) = OPENGL_FUNCTIONS.gl_vertex_2f {
            gl_vertex_2f(x_a, y_a);
            gl_vertex_2f(x_b, y_b);
        }

        if let Some(gl_end) = OPENGL_FUNCTIONS.gl_end {
            gl_end();
        }

        if let Some(gl_enable) = OPENGL_FUNCTIONS.gl_enable {
            gl_enable(GLMODES.gl_texture_2d);
            gl_enable(GLMODES.gl_blend);
        }
    }
    cleanup_draw();
}

pub fn draw_player_skeleton(player: *const Playerent, color: Vec3) {
    unsafe {
        let origin = (&*player).o; // use the enemy's position
        let head = (&*player).head;
        if let (Ok(origin_2d), Ok(head_2d)) = (transform(origin), transform(head)) {
            draw_line(origin_2d.x, origin_2d.y, head_2d.x, head_2d.y, 3.0, color); // red line
        }
    }
}

pub fn draw_player_box(player: *const Playerent, color: Vec3) {
    unsafe {
        let origin = (&*player).o; // use the enemy's position
        let mut head = (&*player).head;
        head.z += 0.5;

        if let (Ok(origin_2d), Ok(head_2d)) = (transform(origin), transform(head)) {
            let half_width = (head_2d.y - origin_2d.y) * 0.5 * 0.5;
            draw_rectangle(
                origin_2d.x - half_width,
                origin_2d.y,
                head_2d.x + half_width,
                head_2d.y,
                color,
            );
        }
    }
}

pub fn draw_player_healthbar(player: *const Playerent) {
    unsafe {
        let origin = (&*player).o; // use the enemy's position
        let head = (&*player).head;
        let health = (&*player).health;

        let normalized_health = health as f32 / 100.0;
        if let (Ok(origin_2d), Ok(head_2d)) = (transform(origin), transform(head)) {
            let x_offset = (head_2d.y - origin_2d.y) * 0.5 * 0.6;

            let healthbar_height = (head_2d.y - origin_2d.y) * normalized_health;
            draw_line(
                origin_2d.x - x_offset,
                origin_2d.y,
                origin_2d.x - x_offset,
                origin_2d.y + healthbar_height,
                3.0,
                Vec3 {
                    x: 255.0,
                    y: 255.0,
                    z: 255.0,
                },
            );
        }
    }
}

pub fn draw_player_traceline(player: *const Playerent, color: Vec3) {
    unsafe {
        let origin = (&*player).o;
        if let Some(worldpos_ptr) = PROCESS.worldpos_ptr {
            let worldpos = *worldpos_ptr;
            if let (Some(screenh_ptr), Ok(mut worldpos_2d), Ok(origin_2d)) =
                (PROCESS.screenh_ptr, transform(worldpos), transform(origin))
            {
                worldpos_2d.y += (*screenh_ptr) as f32 / 2.0;
                draw_line(
                    worldpos_2d.x,
                    worldpos_2d.y,
                    origin_2d.x,
                    origin_2d.y,
                    1.0,
                    color,
                );
            }
        }
    }
}

fn player_esp_class(player1: &Playerent, player: &Playerent) -> Result<i32, Error> {
    let mut state = -1;

    if player.state == 1 {
        return Ok(state);
    } else {
        state = 0;
    }

    if player.team == player1.team {
        state += 2;
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
                if result {
                    state += 1;
                }
            }
            None => return Err(Error::TraceLineError),
        };
    }

    Ok(state)
}

fn is_usable_ent(ent: &PersistentEnt) -> Result<bool, Error> {
    let type_ = ent.type_;
    if type_ > 2 && type_ < 10 && ent.spawned {
        return Ok(true);
    } else {
        return Ok(false);
    }
}

pub fn draw_ent_box(ent: &PersistentEnt, color: Vec3) {
    let origin = Vec3 {
        x: ent.x as f32,
        y: ent.y as f32,
        z: ent.z as f32,
    };

    let upper_bound = Vec3 {
        x: ent.x as f32,
        y: ent.y as f32,
        z: ent.z as f32 - 0.5,
    };

    if let (Ok(origin_2d), Ok(ub_2d)) = (transform(origin), transform(upper_bound)) {
        let half_width = (ub_2d.y - origin_2d.y) * 0.5;
        draw_rectangle(
            origin_2d.x - half_width,
            origin_2d.y,
            ub_2d.x + half_width,
            ub_2d.y,
            color,
        );
    }
}

pub fn draw_entities() -> Result<(), Error> {
    unsafe {
        let ents = match PROCESS.ents_ptr {
            Some(ptr) => ptr,
            None => return Err(Error::EntsListError),
        };

        let ents_length: usize = {
            let length_addr = (ents as u64 + 0xC) as *const u32;
            *length_addr as usize
        };

        let ents_list_ptr = *ents;
        let ents_list = std::slice::from_raw_parts(ents_list_ptr, ents_length);

        ents_list
            .iter()
            .map(|ent| (ent, is_usable_ent(ent)))
            .for_each(|(ent, result)| match result {
                Ok(true) => {
                    let mut color = Vec3 {
                        x: 255.0,
                        y: 255.0,
                        z: 255.0,
                    };
                    if ent.type_ == 4 || ent.type_ == 3 {
                        color.y -= 255.0;
                    }
                    if ent.type_ == 6 {
                        color.x -= 255.0;
                        color.z -= 255.0;
                    }

                    if ent.type_ == 9 {
                        color.x -= 255.0;
                        color.y -= 255.0;
                    }
                    draw_ent_box(ent, color);
                }
                Ok(false) => return,
                Err(_) => return,
            });
    }
    Ok(())
}

pub fn draw_players() -> Result<(), Error> {
    unsafe {
        let player1: &mut Playerent = match PROCESS.player1_ptr {
            Some(ptr) => &mut *ptr,
            None => return Err(Error::Player1Error),
        };

        let players = match PROCESS.players_ptr {
            Some(ptr) => ptr,
            None => return Err(Error::PlayersListError),
        };

        let players_length: usize = {
            let length_addr = (players as u64 + 0xC) as *const u32;
            *length_addr as usize
        };

        let players_list_ptr = *players;

        let players_list = std::slice::from_raw_parts(players_list_ptr.add(1), players_length - 1);
        players_list
            .iter()
            .map(|&ptr| {
                let player = &*(ptr as *const Playerent);
                (player, player_esp_class(player1, player))
            })
            .for_each(|(player, result)| match result {
                Ok(result) => {
                    // enemy
                    if result == 0 {
                        draw_player_box(
                            player,
                            Vec3 {
                                x: 255.0,
                                y: 0.0,
                                z: 0.0,
                            },
                        );
                    }
                    if result == 2 {
                        draw_player_skeleton(
                            player,
                            Vec3 {
                                x: 0.0,
                                y: 255.0,
                                z: 0.0,
                            },
                        );
                    }
                }
                Err(_) => return,
            });
    }
    Ok(())
}
