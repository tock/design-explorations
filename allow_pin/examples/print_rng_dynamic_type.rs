#![no_main]
#![no_std]

use allow_pin::{command, dynamic_type::*};
use core::pin::pin;

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    // Read some random data.
    let mut rng_buffer = pin!(Buffer::<[u8; 8], 0x40001, 0x0>::from([0; 8]));
    rng_buffer.as_mut().allow(DynamicType::Rw)?;
    command(0x40001, 0x1, 8, 0)?;
    // Wait for an upcall here.

    // Retrieve the buffer from the RNG, then write that data to the console.
    let console_buffer = pin!(Buffer::<[u8; 8], 0x1, 0x1>::from(*rng_buffer.unallow()));
    console_buffer.allow(DynamicType::Ro)?;
    command(0x1, 0x1, 8, 0)?;
    // Wait for an upcall here.
    Ok(())
}
