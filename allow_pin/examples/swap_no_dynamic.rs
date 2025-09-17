#![no_main]
#![no_std]

use allow_pin::{ErrorCode, command, no_dynamic::*};
use core::pin::{Pin, pin};

// Over-simplified streaming process client just to prove we can make streaming
// process buffers work.
struct StreamingReceiveSlice<const LEN: usize, const DRIVER_NUM: u32, const BUFFER_NUM: u32> {
    // Two buffers to stream RNG data from. Both buffer_a and buffer_b are
    // structurally pinned fields.
    buffer_a: Buffer<StaticRw, [u8; LEN], DRIVER_NUM, BUFFER_NUM>,
    buffer_b: Buffer<StaticRw, [u8; LEN], DRIVER_NUM, BUFFER_NUM>,

    share_status: ShareStatus,
}

impl<const LEN: usize, const DRIVER_NUM: u32, const BUFFER_NUM: u32> Default
    for StreamingReceiveSlice<LEN, DRIVER_NUM, BUFFER_NUM>
{
    fn default() -> StreamingReceiveSlice<LEN, DRIVER_NUM, BUFFER_NUM> {
        StreamingReceiveSlice {
            buffer_a: Buffer::from([0; LEN]),
            buffer_b: Buffer::from([0; LEN]),
            share_status: ShareStatus::None,
        }
    }
}

impl<const LEN: usize, const DRIVER_NUM: u32, const BUFFER_NUM: u32>
    StreamingReceiveSlice<LEN, DRIVER_NUM, BUFFER_NUM>
{
    // Shares the first buffer, starting the receive.
    pub fn start(mut self: Pin<&mut Self>) -> Result<(), ErrorCode> {
        unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.buffer_a) }.allow()?;
        unsafe { self.get_unchecked_mut() }.share_status = ShareStatus::A;
        Ok(())
    }

    // Called repeatedly to swap the buffers and receive the next chunk.
    pub fn next(self: Pin<&mut Self>) -> Result<&mut [u8; LEN], ErrorCode> {
        let this = unsafe { self.get_unchecked_mut() };
        let [a, b] = unsafe {
            [
                Pin::new_unchecked(&mut this.buffer_a),
                Pin::new_unchecked(&mut this.buffer_b),
            ]
        };
        let (old, new, status) = match this.share_status {
            ShareStatus::None => return Err(4),
            ShareStatus::A => (a, b, ShareStatus::B),
            ShareStatus::B => (b, a, ShareStatus::A),
        };
        let (out, result) = old.replace_with_mut(new);
        result?;
        this.share_status = status;
        Ok(out)
    }
}

// Which buffer is currently shared.
enum ShareStatus {
    None,
    A,
    B,
}

#[unsafe(no_mangle)]
fn _start() -> Result<(), u32> {
    let mut rng_stream = pin!(StreamingReceiveSlice::<8, 0x40001, 0x0>::default());
    rng_stream.as_mut().start()?;
    command(0x40001, 0x1, 8, 0)?;
    // Wait for an upcall here.
    let _random1 = *rng_stream.as_mut().next()?;
    command(0x40001, 0x1, 8, 0)?;
    // Wait for an upcall here.
    let _random2 = *rng_stream.next()?;
    Ok(())
}
