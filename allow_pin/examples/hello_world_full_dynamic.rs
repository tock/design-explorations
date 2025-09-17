#![no_main]
#![no_std]

use allow_pin::{command, full_dynamic::*};
use core::pin::pin;

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    let mut buffer = pin!(Buffer::<[u8; _]>::from(*b"hi"));
    buffer.as_mut().allow(DynamicType::Ro, 0x1, 0x1)?;

    command(0x1, 0x1, 14, 0)?;
    // Wait for an upcall here.
    Ok(())
}
