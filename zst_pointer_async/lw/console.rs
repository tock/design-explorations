/// Console driver. Only supports writing to the console.

use core::ptr::null_mut;
use crate::lw::async_util::Forwarder;
use crate::returncode_subset;
use crate::syscalls::{allow_ptr, command, subscribe_ptr};

const DRIVER_NUM: usize = 1;
const WRITE: usize = 1;
const WRITE_COMPLETE: usize = 1;
const WRITE_BUFFER: usize = 1;

pub type Buffer = &'static mut [u8];

// TODO: Should we have an abstraction for drivers that just hand a buffer to
// the kernel and wait for a callback? This is the same logic as the RNG driver.

// TODO: Evaluate the distinctions between ENODEVICE, ENOMEM, and FAIL.

pub struct Console<F: Forwarder<Option<Buffer>>> {
    buffer_data: core::cell::Cell<*mut u8>,
    buffer_len: core::cell::Cell<usize>,

    forwarder: F,
}

impl<F: Forwarder<Option<Buffer>>> Console<F> {
    pub const fn new(forwarder: F) -> Console<F> {
        Console {
            buffer_data: core::cell::Cell::new(core::ptr::null_mut()),
            buffer_len: core::cell::Cell::new(0),
            forwarder
        }
    }

    pub fn write(&'static self, buffer: Buffer) -> Result<(), (WriteError, Option<Buffer>)> {
        if !self.buffer_data.get().is_null() { return Err((WriteError::EBUSY, Some(buffer))); }
        match unsafe { subscribe_ptr(DRIVER_NUM, WRITE_COMPLETE, callback::<F> as *const _,
                                     self as *const Self as usize) } {
            0 => {},  // Success
            crate::result::ENOMEM => return Err((WriteError::ENOMEM, Some(buffer))),
            crate::result::ENODEVICE => return Err((WriteError::ENODEVICE, Some(buffer))),
            _ => return Err((WriteError::FAIL, Some(buffer))),
        }
        if unsafe { allow_ptr(DRIVER_NUM, WRITE_BUFFER, buffer.as_mut_ptr(), buffer.len()) } != 0 {
            return Err((WriteError::FAIL, Some(buffer)));
        }
        if unsafe { command(DRIVER_NUM, WRITE, buffer.len(), 0) } != 0 {
            // Unable to start the fetch, but we've already allow()-ed the
            // buffer to the kernel. Try to get it back.
            return match unsafe { allow_ptr(DRIVER_NUM, WRITE_BUFFER, null_mut(), 0) } {
                0 => Err((WriteError::FAIL, Some(buffer))),
                _ => Err((WriteError::FAIL, None)),
            }
        }
        self.buffer_data.set(buffer.as_mut_ptr());
        self.buffer_len.set(buffer.len());
        Ok(())
    }
}

returncode_subset![ pub enum WriteError { FAIL, EBUSY, ENODEVICE, ENOMEM } ];

unsafe extern fn callback<F: Forwarder<Option<Buffer>>>(_bytes_written: usize, _: usize, _: usize, console: usize) {
    let console = &*(console as *const Console<F>);

    if allow_ptr(DRIVER_NUM, WRITE_BUFFER, null_mut(), 0) != 0 {
        // Failed to un-allow the buffer. We leave the buffer values set in
        // Console, which puts it into a "poisoned" state where it will return
        // BUSY forever.
        console.forwarder.invoke_callback(None);
    }
    console.forwarder.invoke_callback(Some(
        core::slice::from_raw_parts_mut(console.buffer_data.replace(null_mut()), console.buffer_len.get())
    ));
}
