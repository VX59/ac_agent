use crate::agent_utils::{Playerent, Traceresults, WorldPos, process_next_target, ray_scan};
use crate::aimbot_utils::{get_best_viewangles, update_agent_viewangles};
use crate::err::Error;
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

pub struct AcFunctions {
    pub trace_line_func: Option<TracelineFn>,
    pub is_visible_func: Option<IsVisibleFn>,
}

pub static mut AC_FUNCTIONS: AcFunctions = AcFunctions {
    trace_line_func: None,
    is_visible_func: None,
};

pub struct Process {
    pub player1_ptr: Option<*mut Playerent>,
    pub players_ptr: Option<*const *const u64>,
}

pub static mut PROCESS: Process = Process {
    player1_ptr: None,
    players_ptr: None,
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
        let _ = update_agent_viewangles();
        let _ = process_next_target();
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

pub fn get_symbol_offset(symbol: &str) -> anyhow::Result<u64> {
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

pub fn init_hooks(native_client_addr: u64) -> Result<(), Error> {
    unsafe {
        let players_offset = get_symbol_offset("players");

        let players_addr = match players_offset {
            Ok(offset) => {
                println!("players offset @ {:#X}", offset);
                Some(native_client_addr + offset)
            }
            Err(_) => return Err(Error::SymbolError),
        };

        PROCESS.players_ptr = match players_addr {
            Some(addr) => {
                let ptr = addr as *const *const u64;
                Some(ptr)
            }
            None => return Err(Error::PlayersListError),
        };

        let player1_offset = get_symbol_offset("player1");

        let player1_addr = match player1_offset {
            Ok(offset) => {
                println!("player1 offset @ {:#X}", offset);
                Some(native_client_addr + offset)
            }
            Err(_) => return Err(Error::SymbolError),
        };

        PROCESS.player1_ptr = match player1_addr {
            Some(addr) => {
                let ptr = addr as *const *mut Playerent;
                Some(*ptr)
            }
            None => return Err(Error::Player1Error),
        };

        let trace_line_offset = get_symbol_offset("_Z9TraceLine3vecS_P6dynentbP13traceresult_sb");
        let trace_line_addr = match trace_line_offset {
            Ok(offset) => (native_client_addr + offset) as usize,
            Err(_) => return Err(Error::SymbolError),
        };

        AC_FUNCTIONS.trace_line_func = Some(mem::transmute::<usize, TracelineFn>(trace_line_addr));

        let is_visible_offset = get_symbol_offset("_Z9IsVisible3vecS_P6dynentb");
        let is_visible_addr = match is_visible_offset {
            Ok(offset) => (native_client_addr + offset) as usize,
            Err(_) => return Err(Error::SymbolError),
        };

        AC_FUNCTIONS.is_visible_func = Some(mem::transmute::<usize, IsVisibleFn>(is_visible_addr));

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
