#![no_main]
#![no_std]

use allow_pin::{command, no_dynamic::*};
use core::pin::pin;

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    let mut buffer = pin!(Buffer::<StaticRw, [u8; 8], 0x40001, 0x0>::from([0; 8]));
    buffer.as_mut().allow()?;
    command(0x40001, 0x1, 8, 0)?;
    // Wait for an upcall here.
    let _random = *buffer.as_ref().buffer();
    Ok(())
}
