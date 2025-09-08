//! An example that makes many different system calls with many different buffer
//! sizes (using generics and lots of random numbers). Intended to test how the
//! code size scales to large apps that use many drivers.

#![no_main]
#![no_std]

use allow_pin::{ErrorCode, command, no_dynamic::*};
use core::pin::{Pin, pin};

/// Using the specified driver number, write the given buffer to the given RO
/// Allow ID and read the given buffer from the given RW Allow ID, returning its
/// contents. This intentionally does not know the buffer sizes at compile time,
/// as it's simulating an API working on data provided by an external crate.
fn api<const DRIVER_NUM: u32, const RO_BUFFER: u32, const RW_BUFFER: u32>(
    ro_buffer: Pin<&Buffer<StaticRo, [u8], DRIVER_NUM, RO_BUFFER>>,
    mut rw_buffer: Pin<&mut Buffer<StaticRw, [u8], DRIVER_NUM, RW_BUFFER>>,
) -> Result<(), ErrorCode> {
    ro_buffer.allow_ro()?;
    rw_buffer.as_mut().allow()?;
    // Subscribe goes here.
    // Dummy command invocation to clobber registers and add an error return
    // path.
    command(DRIVER_NUM, RO_BUFFER, RW_BUFFER, 0)?;
    // Yield goes here.
    // Perform unallows.
    ro_buffer.buffer();
    rw_buffer.into_ref().buffer();
    Ok(())
}

/// Pretend application-crate function that creates the allow buffers and then
/// uses them with api().
fn app<
    const DRIVER_NUM: u32,
    const RO_BUFFER: u32,
    const RW_BUFFER: u32,
    const RO_LEN: usize,
    const RW_LEN: usize,
>(
    ro_data: [u8; RO_LEN],
) -> Result<[u8; RW_LEN], ErrorCode> {
    let ro_buffer = pin!(Buffer::<_, [u8; RO_LEN], _, _>::from(ro_data));
    let mut rw_buffer = pin!(Buffer::from([0; RW_LEN]));
    api::<DRIVER_NUM, RO_BUFFER, RW_BUFFER>(ro_buffer, rw_buffer.as_mut())?;
    Ok(*rw_buffer.into_ref().buffer())
}

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    let buffer = [0; 40];
    let buffer: [_; 81] = app::<8198, 9, 9, _, _>(buffer)?;
    let buffer: [_; 70] = app::<8388, 7, 0, _, _>(buffer)?;
    let buffer: [_; 24] = app::<9167, 3, 2, _, _>(buffer)?;
    let buffer: [_; 54] = app::<0543, 0, 7, _, _>(buffer)?;
    let buffer: [_; 37] = app::<5527, 7, 0, _, _>(buffer)?;
    let buffer: [_; 14] = app::<1639, 2, 8, _, _>(buffer)?;
    let buffer: [_; 72] = app::<4817, 3, 8, _, _>(buffer)?;
    let buffer: [_; 15] = app::<4381, 8, 9, _, _>(buffer)?;
    let buffer: [_; 37] = app::<0162, 8, 0, _, _>(buffer)?;
    let buffer: [_; 42] = app::<2364, 0, 3, _, _>(buffer)?;
    let buffer: [_; 99] = app::<7292, 4, 0, _, _>(buffer)?;
    let buffer: [_; 39] = app::<7764, 7, 0, _, _>(buffer)?;
    let buffer: [_; 30] = app::<4322, 3, 3, _, _>(buffer)?;
    let buffer: [_; 44] = app::<1019, 7, 0, _, _>(buffer)?;
    let buffer: [_; 37] = app::<1919, 3, 0, _, _>(buffer)?;
    let buffer: [_; 24] = app::<2672, 9, 8, _, _>(buffer)?;
    let buffer: [_; 54] = app::<0243, 8, 4, _, _>(buffer)?;
    let buffer: [_; 52] = app::<9158, 4, 2, _, _>(buffer)?;
    let buffer: [_; 46] = app::<0521, 2, 8, _, _>(buffer)?;
    let buffer: [_; 56] = app::<2717, 9, 9, _, _>(buffer)?;
    let buffer: [_; 52] = app::<0302, 8, 3, _, _>(buffer)?;
    let buffer: [_; 26] = app::<4812, 0, 5, _, _>(buffer)?;
    let buffer: [_; 80] = app::<2798, 7, 4, _, _>(buffer)?;
    let buffer: [_; 60] = app::<3450, 3, 9, _, _>(buffer)?;
    let buffer: [_; 50] = app::<6942, 6, 8, _, _>(buffer)?;
    let buffer: [_; 50] = app::<7943, 3, 1, _, _>(buffer)?;
    let buffer: [_; 16] = app::<6614, 4, 9, _, _>(buffer)?;
    let buffer: [_; 54] = app::<6537, 1, 7, _, _>(buffer)?;
    let buffer: [_; 15] = app::<1619, 5, 3, _, _>(buffer)?;
    let buffer: [_; 19] = app::<7755, 3, 0, _, _>(buffer)?;
    let buffer: [_; 85] = app::<0814, 9, 7, _, _>(buffer)?;
    let buffer: [_; 50] = app::<8341, 8, 8, _, _>(buffer)?;
    let buffer: [_; 65] = app::<2333, 7, 5, _, _>(buffer)?;
    let buffer: [_; 34] = app::<9340, 1, 0, _, _>(buffer)?;
    let buffer: [_; 81] = app::<1374, 9, 9, _, _>(buffer)?;
    let buffer: [_; 79] = app::<3606, 2, 4, _, _>(buffer)?;
    let buffer: [_; 59] = app::<2566, 8, 5, _, _>(buffer)?;
    let buffer: [_; 39] = app::<3027, 5, 3, _, _>(buffer)?;
    let buffer: [_; 20] = app::<8905, 6, 5, _, _>(buffer)?;
    let buffer: [_; 43] = app::<9119, 8, 4, _, _>(buffer)?;
    let buffer: [_; 30] = app::<2019, 7, 8, _, _>(buffer)?;
    let buffer: [_; 81] = app::<6507, 2, 0, _, _>(buffer)?;
    let buffer: [_; 15] = app::<5055, 0, 3, _, _>(buffer)?;
    let buffer: [_; 70] = app::<7550, 9, 2, _, _>(buffer)?;
    let buffer: [_; 86] = app::<7760, 7, 3, _, _>(buffer)?;
    let buffer: [_; 73] = app::<7275, 6, 6, _, _>(buffer)?;
    let buffer: [_; 35] = app::<5457, 7, 1, _, _>(buffer)?;
    let buffer: [_; 43] = app::<3421, 2, 0, _, _>(buffer)?;
    let buffer: [_; 13] = app::<5221, 6, 0, _, _>(buffer)?;
    let buffer: [_; 33] = app::<5808, 0, 0, _, _>(buffer)?;
    let buffer: [_; 83] = app::<9534, 2, 8, _, _>(buffer)?;
    let buffer: [_; 77] = app::<5818, 9, 4, _, _>(buffer)?;
    let buffer: [_; 60] = app::<8619, 4, 9, _, _>(buffer)?;
    let buffer: [_; 56] = app::<7449, 3, 3, _, _>(buffer)?;
    let buffer: [_; 39] = app::<3627, 2, 8, _, _>(buffer)?;
    let buffer: [_; 81] = app::<2614, 0, 4, _, _>(buffer)?;
    let buffer: [_; 79] = app::<8430, 4, 9, _, _>(buffer)?;
    let buffer: [_; 97] = app::<0438, 2, 8, _, _>(buffer)?;
    let buffer: [_; 30] = app::<0392, 4, 6, _, _>(buffer)?;
    let buffer: [_; 33] = app::<6581, 6, 9, _, _>(buffer)?;
    let buffer: [_; 42] = app::<5619, 1, 3, _, _>(buffer)?;
    let buffer: [_; 89] = app::<3794, 1, 3, _, _>(buffer)?;
    let buffer: [_; 74] = app::<5252, 0, 4, _, _>(buffer)?;
    let buffer: [_; 45] = app::<3645, 2, 3, _, _>(buffer)?;
    let buffer: [_; 83] = app::<3779, 2, 7, _, _>(buffer)?;
    let buffer: [_; 58] = app::<9797, 6, 7, _, _>(buffer)?;
    let buffer: [_; 93] = app::<5284, 0, 5, _, _>(buffer)?;
    let buffer: [_; 64] = app::<4136, 8, 4, _, _>(buffer)?;
    let buffer: [_; 49] = app::<4046, 8, 5, _, _>(buffer)?;
    let buffer: [_; 72] = app::<6158, 4, 3, _, _>(buffer)?;
    let buffer: [_; 41] = app::<9892, 3, 4, _, _>(buffer)?;
    let buffer: [_; 26] = app::<8264, 5, 3, _, _>(buffer)?;
    let buffer: [_; 27] = app::<7374, 5, 2, _, _>(buffer)?;
    let buffer: [_; 65] = app::<0320, 8, 9, _, _>(buffer)?;
    let buffer: [_; 24] = app::<8534, 1, 9, _, _>(buffer)?;
    let buffer: [_; 30] = app::<3259, 9, 5, _, _>(buffer)?;
    let buffer: [_; 60] = app::<2876, 2, 3, _, _>(buffer)?;
    let buffer: [_; 44] = app::<7852, 5, 7, _, _>(buffer)?;
    let buffer: [_; 39] = app::<4533, 4, 3, _, _>(buffer)?;
    let buffer: [_; 79] = app::<2892, 9, 0, _, _>(buffer)?;
    let buffer: [_; 53] = app::<5847, 7, 1, _, _>(buffer)?;
    let buffer: [_; 63] = app::<3671, 0, 3, _, _>(buffer)?;
    let buffer: [_; 17] = app::<5335, 4, 3, _, _>(buffer)?;
    let buffer: [_; 63] = app::<3248, 7, 4, _, _>(buffer)?;
    let buffer: [_; 28] = app::<5780, 5, 9, _, _>(buffer)?;
    let buffer: [_; 93] = app::<3134, 8, 5, _, _>(buffer)?;
    let buffer: [_; 18] = app::<5617, 0, 2, _, _>(buffer)?;
    let buffer: [_; 68] = app::<9266, 2, 9, _, _>(buffer)?;
    let buffer: [_; 35] = app::<2495, 6, 3, _, _>(buffer)?;
    let buffer: [_; 36] = app::<7126, 4, 8, _, _>(buffer)?;
    let buffer: [_; 47] = app::<4723, 6, 9, _, _>(buffer)?;
    let buffer: [_; 94] = app::<0555, 5, 2, _, _>(buffer)?;
    let buffer: [_; 85] = app::<6026, 7, 3, _, _>(buffer)?;
    let buffer: [_; 17] = app::<3209, 2, 2, _, _>(buffer)?;
    let buffer: [_; 80] = app::<6825, 6, 5, _, _>(buffer)?;
    let buffer: [_; 27] = app::<0985, 1, 2, _, _>(buffer)?;
    let buffer: [_; 99] = app::<9544, 3, 3, _, _>(buffer)?;
    let buffer: [_; 44] = app::<0024, 5, 3, _, _>(buffer)?;
    let buffer: [_; 42] = app::<9456, 3, 0, _, _>(buffer)?;
    let buffer: [_; 40] = app::<7930, 4, 8, _, _>(buffer)?;
    let _ = buffer;
    Ok(())
}
