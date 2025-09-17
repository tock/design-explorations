#![no_main]
#![no_std]

use allow_pin::{command, dynamic_type::*};
use core::pin::pin;

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    // I wanted to use "Hello, world!\n" but long strings resulted in memcpy
    // being included, which is hundreds of bytes and throws off the code size
    // comparison.
    let mut buffer = pin!(Buffer::<[u8; _], 0x1, 0x1>::from(*b"hi"));
    buffer.as_mut().allow(DynamicType::Ro)?;
    command(0x1, 0x1, 14, 0)?;
    // Wait for an upcall here.
    Ok(())
}
