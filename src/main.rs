#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(panic_info_message, alloc_error_handler)]
#![feature(concat_idents)]

#[macro_use]
extern crate log;

extern crate alloc;

#[macro_use]
mod logging;

mod arch;
mod config;
mod hv;
mod mm;
mod timer;

#[cfg(not(test))]
mod lang_items;

use core::sync::atomic::{AtomicBool, Ordering};

static INIT_OK: AtomicBool = AtomicBool::new(false);

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

pub fn init_ok() -> bool {
    INIT_OK.load(Ordering::SeqCst)
}

fn main() -> () {
    clear_bss();
    arch::init_early();
    println!(
        "\
        arch = {}\n\
        build_mode = {}\n\
        log_level = {}\n\
        ",
        option_env!("ARCH").unwrap_or(""),
        option_env!("MODE").unwrap_or(""),
        option_env!("LOG").unwrap_or(""),
    );

    mm::init_heap_early();
    logging::init();
    info!("Logging is enabled.");

    arch::init();
    mm::init();
    INIT_OK.store(true, Ordering::SeqCst);
    println!("Initialization completed.\n");

    hv::run();
}
