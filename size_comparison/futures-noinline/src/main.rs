//! Futures-based test app. Blinks a light. While a button is pressed, the
//! blinking is suspended. Note that golf2 doesn't implement the LED or button
//! syscall drivers, so these are all just GPIO calls.

#![no_std]
#![feature(asm,const_fn,lang_items,naked_functions)]

mod alarm;
mod app;
mod entry_point;
mod gpio;
mod lang_items;
mod syscalls;
mod task;
mod tock_static;

fn main() {
    alarm::init();
    gpio::start();
    app::APP.start();

    loop {
        syscalls::yieldk();
    }
}
