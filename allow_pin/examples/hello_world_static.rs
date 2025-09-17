#![no_main]
#![no_std]

use allow_pin::{command, full_dynamic::*};
use core::pin::pin;

static WELCOME: [u8; 2] = *b"hi";

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    let mut buffer = pin!(StaticBuffer::<[u8; _]>::from(&WELCOME));
    buffer.as_mut().allow(0x1, 0x1)?;

    command(0x1, 0x1, 14, 0)?;
    // Wait for an upcall here.
    Ok(())
}
