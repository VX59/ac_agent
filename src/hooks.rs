use crate::agent_utils::{
    PersistentEnt, Playerent, Traceresults, Vec3, WorldPos, process_next_target, turn_off_p1_recoil,
};
use crate::aimbot_utils::update_agent_viewangles;
use crate::err::Error;
use crate::esp::{draw_entities, draw_players};
use anyhow::Result;
use anyhow::anyhow;
use goblin::elf::Elf;
use libc::{RTLD_LAZY, c_char, c_int, dl_iterate_phdr, dl_phdr_info, dlopen, dlsym, size_t};
use std::ffi::c_void;
use std::fs;
use std::mem;
use std::ptr::read_unaligned;

use log::debug;

type SdlGLSwapWindowInnerFn = unsafe extern "C" fn(*const c_void);

type TracelineFn = unsafe extern "C" fn(WorldPos, WorldPos, u64, bool, *const Traceresults);
type IsVisibleFn = unsafe extern "C" fn(WorldPos, WorldPos, u64, bool) -> bool;
type DrawTextFn = unsafe extern "C" fn(*const c_char, i32, i32, i32, i32, i32, i32, i32, i32);

type GlBeginFn = unsafe extern "C" fn(i32);
type GlEndFn = unsafe extern "C" fn();
type GlVertex2fFn = unsafe extern "C" fn(f32, f32);
type GlLineWidthFn = unsafe extern "C" fn(f32);
type GlColor3fFn = unsafe extern "C" fn(f32, f32, f32);
type GlEnableFn = unsafe extern "C" fn(u32);
type GlDisableFn = unsafe extern "C" fn(u32);
type GlOrthoFn = unsafe extern "C" fn(f64, f64, f64, f64, f64, f64);
type GlPushMatrixFn = unsafe extern "C" fn();
type GlLoadIdentityFn = unsafe extern "C" fn();
type GlMatrixModeFn = unsafe extern "C" fn(i32);
type GlPopMatrixFn = unsafe extern "C" fn();

pub struct AcFunctions {
    pub trace_line_func: Option<TracelineFn>,
    pub is_visible_func: Option<IsVisibleFn>,
    pub draw_text_func: Option<DrawTextFn>,
}

pub static mut AC_FUNCTIONS: AcFunctions = AcFunctions {
    trace_line_func: None,
    is_visible_func: None,
    draw_text_func: None,
};

pub struct OpenglFunctions {
    pub gl_begin: Option<GlBeginFn>,
    pub gl_end: Option<GlEndFn>,
    pub gl_vertex_2f: Option<GlVertex2fFn>,
    pub gl_line_width: Option<GlLineWidthFn>,
    pub gl_color_3f: Option<GlColor3fFn>,
    pub gl_enable: Option<GlEnableFn>,
    pub gl_disable: Option<GlDisableFn>,

    pub gl_ortho: Option<GlOrthoFn>,
    pub gl_matrix_mode: Option<GlMatrixModeFn>,
    pub gl_push_matrix: Option<GlPushMatrixFn>,
    pub gl_load_identity: Option<GlLoadIdentityFn>,
    pub gl_pop_matrix: Option<GlPopMatrixFn>,
}

pub static mut OPENGL_FUNCTIONS: OpenglFunctions = OpenglFunctions {
    gl_begin: None,
    gl_end: None,
    gl_vertex_2f: None,
    gl_line_width: None,
    gl_color_3f: None,
    gl_enable: None,
    gl_disable: None,

    gl_ortho: None,
    gl_matrix_mode: None,
    gl_push_matrix: None,
    gl_load_identity: None,
    gl_pop_matrix: None,
};

pub struct Process {
    pub player1_ptr: Option<*mut Playerent>,
    pub players_ptr: Option<*const *const u64>,
    pub ents_ptr: Option<*const *const PersistentEnt>,
    pub mvpmatrix_ptr: Option<*const f32>,
    pub screenw_ptr: Option<*const i32>,
    pub screenh_ptr: Option<*const i32>,
    pub worldpos_ptr: Option<*const Vec3>,
}

pub static mut PROCESS: Process = Process {
    player1_ptr: None,
    players_ptr: None,
    ents_ptr: None,
    mvpmatrix_ptr: None,
    screenw_ptr: None,
    screenh_ptr: None,
    worldpos_ptr: None,
};

static mut MUTABLE_INNER_FUNC_PTR: Option<*mut unsafe extern "C" fn(*const c_void)> = None;
static mut HOOK_ORIGINAL_INNER_FUNC: Option<SdlGLSwapWindowInnerFn> = None;

macro_rules! cstr_static {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe extern "C" fn hook_func(window: *const c_void) {
    unsafe {
        let _ = draw_players();
        let _ = process_next_target();

        let _ = update_agent_viewangles();

        let _ = draw_entities();
        match HOOK_ORIGINAL_INNER_FUNC {
            Some(func) => func(window),
            None => (),
        }
    }
}

pub fn init_sdl_gl_swap_window_hook(sdl_gl_swap_window_handle: *mut c_void) -> Result<(), Error> {
    unsafe {
        let wrapper_offset_location = sdl_gl_swap_window_handle as u64 + 0x4 + 0x2;

        let offset = read_unaligned(wrapper_offset_location as *const u32);

        MUTABLE_INNER_FUNC_PTR = Some(
            (sdl_gl_swap_window_handle as u64 + 0xa + offset as u64)
                as *mut unsafe extern "C" fn(*const c_void),
        );

        match MUTABLE_INNER_FUNC_PTR {
            Some(ptr) => {
                HOOK_ORIGINAL_INNER_FUNC = Some(*ptr);
                *(ptr) = hook_func;
            }
            None => return Err(Error::SDLHookError),
        };
    }
    Ok(())
}

pub fn recover_sdl_gl_swap_window() -> Result<(), Error> {
    unsafe {
        match MUTABLE_INNER_FUNC_PTR {
            Some(ptr) => {
                *(ptr) = match HOOK_ORIGINAL_INNER_FUNC {
                    Some(ptr) => ptr,
                    None => return Err(Error::SDLHookError),
                }
            }
            None => return Err(Error::SDLHookError),
        };
        println!("unhooked successfully");
        Ok(())
    }
}

fn get_symbol_offset(symbol: &str) -> anyhow::Result<u64> {
    let bin = fs::read("/proc/self/exe")?;
    let elf = Elf::parse(&bin)?;

    for sym in &elf.syms {
        if let Some(name) = elf.strtab.get_at(sym.st_name) {
            if name == symbol {
                debug!("Found symbol {} @ {:#X}", symbol, sym.st_value);
                return Ok(sym.st_value);
            }
        }
    }

    Err(anyhow!("Failed to find symbol {}", symbol))
}

fn get_symbol_address(base_addr: u64, symbol: &str) -> Result<u64, Error> {
    let fn_offset = get_symbol_offset(symbol);
    match fn_offset {
        Ok(offset) => Ok(base_addr + offset),
        Err(_) => return Err(Error::SymbolError),
    }
}

pub fn init_hooks(native_client_addr: u64) -> Result<(), Error> {
    unsafe {
        PROCESS.players_ptr =
            Some(get_symbol_address(native_client_addr, "players")? as *const *const u64);

        PROCESS.ents_ptr =
            Some(get_symbol_address(native_client_addr, "ents")? as *const *const PersistentEnt);

        PROCESS.player1_ptr =
            Some(*(get_symbol_address(native_client_addr, "player1")? as *const *mut Playerent));

        PROCESS.mvpmatrix_ptr =
            Some(get_symbol_address(native_client_addr, "mvpmatrix")? as *const f32);

        PROCESS.screenw_ptr =
            Some(get_symbol_address(native_client_addr, "screenw")? as *const i32);

        PROCESS.screenh_ptr =
            Some(get_symbol_address(native_client_addr, "screenh")? as *const i32);

        PROCESS.worldpos_ptr =
            Some(get_symbol_address(native_client_addr, "worldpos")? as *const Vec3);

        AC_FUNCTIONS.trace_line_func =
            Some(mem::transmute::<u64, TracelineFn>(get_symbol_address(
                native_client_addr,
                "_Z9TraceLine3vecS_P6dynentbP13traceresult_sb",
            )?));

        AC_FUNCTIONS.is_visible_func = Some(mem::transmute::<u64, IsVisibleFn>(
            get_symbol_address(native_client_addr, "_Z9IsVisible3vecS_P6dynentb")?,
        ));

        AC_FUNCTIONS.draw_text_func = Some(mem::transmute::<u64, DrawTextFn>(get_symbol_address(
            native_client_addr,
            "_Z9draw_textPKciiiiiiii",
        )?));

        let opengl_lib_handle: *mut c_void = dlopen(cstr_static!("libGL.so.1.7.0"), RTLD_LAZY);

        if opengl_lib_handle.is_null() {
            return Err(Error::DlOpenError);
        }

        OPENGL_FUNCTIONS.gl_begin = Some(mem::transmute::<*const c_void, GlBeginFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glBegin"),
        )));
        OPENGL_FUNCTIONS.gl_end = Some(mem::transmute::<*const c_void, GlEndFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glEnd"),
        )));
        OPENGL_FUNCTIONS.gl_vertex_2f = Some(mem::transmute::<*const c_void, GlVertex2fFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glVertex2f"),
        )));
        OPENGL_FUNCTIONS.gl_line_width = Some(mem::transmute::<*const c_void, GlLineWidthFn>(
            dlsym(opengl_lib_handle, cstr_static!("glLineWidth")),
        ));
        OPENGL_FUNCTIONS.gl_color_3f = Some(mem::transmute::<*const c_void, GlColor3fFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glColor3f"),
        )));
        OPENGL_FUNCTIONS.gl_enable = Some(mem::transmute::<*const c_void, GlEnableFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glEnable"),
        )));
        OPENGL_FUNCTIONS.gl_disable = Some(mem::transmute::<*const c_void, GlDisableFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glDisable"),
        )));

        OPENGL_FUNCTIONS.gl_ortho = Some(mem::transmute::<*const c_void, GlOrthoFn>(dlsym(
            opengl_lib_handle,
            cstr_static!("glOrtho"),
        )));
        OPENGL_FUNCTIONS.gl_matrix_mode = Some(mem::transmute::<*const c_void, GlMatrixModeFn>(
            dlsym(opengl_lib_handle, cstr_static!("glMatrixMode")),
        ));
        OPENGL_FUNCTIONS.gl_push_matrix = Some(mem::transmute::<*const c_void, GlPushMatrixFn>(
            dlsym(opengl_lib_handle, cstr_static!("glPushMatrix")),
        ));
        OPENGL_FUNCTIONS.gl_load_identity =
            Some(mem::transmute::<*const c_void, GlLoadIdentityFn>(dlsym(
                opengl_lib_handle,
                cstr_static!("glLoadIdentity"),
            )));
        OPENGL_FUNCTIONS.gl_pop_matrix = Some(mem::transmute::<*const c_void, GlPopMatrixFn>(
            dlsym(opengl_lib_handle, cstr_static!("glPopMatrix")),
        ));

        let sdl_lib_handle: *mut c_void = dlopen(cstr_static!("libSDL2-2.0.so"), RTLD_LAZY);

        if sdl_lib_handle.is_null() {
            return Err(Error::DlOpenError);
        }

        let sdl_gl_swap_window_handle = dlsym(sdl_lib_handle, cstr_static!("SDL_GL_SwapWindow"));
        init_sdl_gl_swap_window_hook(sdl_gl_swap_window_handle)?;

        Ok(())
    }
}

pub fn find_base_address() -> Result<u64, Error> {
    extern "C" fn callback(
        info: *mut dl_phdr_info,
        _size: size_t,
        data: *mut libc::c_void,
    ) -> c_int {
        let base_address = data as *mut u64;

        unsafe {
            let info = &*info;
            if info.dlpi_name.is_null() || *info.dlpi_name == 0 {
                *base_address = info.dlpi_addr;
                1
            } else {
                0
            }
        }
    }

    let mut base_address: u64 = 0;
    unsafe {
        dl_iterate_phdr(Some(callback), &mut base_address as *mut u64 as *mut c_void);
    }

    match base_address {
        0 => Err(Error::FindBaseAddrError),
        _ => Ok(base_address),
    }
}
