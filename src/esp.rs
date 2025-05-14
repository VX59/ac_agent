use crate::hooks::OPENGL_FUNCTIONS;

pub struct GlModes {
    gl_blend: u32,
    gl_texture_2d: u32,
    gl_line_loop: i32,
}

static GLMODES: GlModes = GlModes {
    gl_blend: 0x0BE2,
    gl_texture_2d: 0x0DE1,
    gl_line_loop: 0x0002,
};

pub fn draw_rectangle(top_left_x: f32, top_left_y: f32, bottom_right_x: f32, bottom_right_y: f32) {
    unsafe {
        if let Some(gl_disable) = OPENGL_FUNCTIONS.gl_disable {
            gl_disable(GLMODES.gl_blend);
            gl_disable(GLMODES.gl_texture_2d);
        }

        if let Some(gl_color_3f) = OPENGL_FUNCTIONS.gl_color_3f {
            gl_color_3f(255.0, 0.0, 0.0);
        }

        if let Some(gl_line_width) = OPENGL_FUNCTIONS.gl_line_width {
            gl_line_width(1.0);
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

        if let Some(gl_enable) = OPENGL_FUNCTIONS.gl_enable {
            gl_enable(GLMODES.gl_texture_2d);
            gl_enable(GLMODES.gl_blend);
        }
    }
}
