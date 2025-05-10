use err::Error;
use hooks::{find_base_address, init_hooks, recover_sdl_gl_swap_window};

mod agent_utils;
mod aimbot_utils;
mod err;
mod hooks;
mod sdl;

#[used]
#[unsafe(link_section = ".init_array")]
static INIT: extern "C" fn() = {
    extern "C" fn init_wrapper() {
        match init() {
            Err(e) => eprintln!("Error during initialization: {:?}", e),
            _ => (),
        }
    }
    init_wrapper
};

fn init() -> Result<(), Error> {
    let native_client_addr: u64 = find_base_address()?;
    env_logger::init();
    init_hooks(native_client_addr)?;

    Ok(())
}

#[used]
#[unsafe(link_section = ".fini_array")]
static FINI: extern "C" fn() = {
    extern "C" fn fini_wrapper() {
        match fini() {
            Err(e) => eprintln!("Error during hook recovery: {:?}", e),
            _ => (),
        }
    }
    fini_wrapper
};

fn fini() -> Result<(), Error> {
    recover_sdl_gl_swap_window()
}
