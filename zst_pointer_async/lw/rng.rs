//! Interface to the RNG capsule. Fills a provided buffer asynchronously with
//! random bytes, then passes it back to the caller. Does not provide
//! virtualization.
//!
//! RNG has a generic argument (ClientLink) which allows RNG to perform
//! callbacks on its client.

use core::cell::Cell;
use core::ptr::null_mut;
use crate::lw::async_util::Forwarder;
use crate::syscalls::{allow_ptr, command, subscribe_ptr};

const BUFFER_NUM: usize = 0;
const DRIVER_NUM: usize = 0x40001;
const GET_BYTES: usize = 1;
const GET_BYTES_DONE: usize = 0;

pub type Buffer = &'static mut [u8];

/// The RNG driver itself.
pub struct Rng<F: Forwarder<Option<Buffer>>> {
    // The buffer corresponding to an ongoing fetch. buffer_data is null if
    // there is no ongoing fetch. Stored as a raw pointer and length to avoid
    // the undefined behavior that would result from holding a &[u8] pointing to
    // data the kernel is mutating.
    buffer_data: Cell<*mut u8>,
    buffer_len: Cell<usize>,

    forwarder: F,
}

impl<F: Forwarder<Option<Buffer>>> Rng<F> {
    pub const fn new(forwarder: F) -> Rng<F> {
        Rng { buffer_data: Cell::new(null_mut()), buffer_len: Cell::new(0), forwarder }
    }

    pub fn fetch(&'static self, buffer: Buffer) -> Result<(), (FetchError, Option<Buffer>)> {
        if !self.buffer_data.get().is_null() { return Err((FetchError::EBUSY, Some(buffer))); }
        match unsafe { subscribe_ptr(DRIVER_NUM, GET_BYTES_DONE, callback::<F> as *const _,
                                     self as *const Self as usize) } {
            0 => {},  // Success
            -11 => return Err((FetchError::ENODEVICE, Some(buffer))),
            _ => return Err((FetchError::FAIL, Some(buffer))),
        }
        if unsafe { allow_ptr(DRIVER_NUM, BUFFER_NUM, buffer.as_mut_ptr(), buffer.len()) } != 0 {
            return Err((FetchError::FAIL, Some(buffer)));
        }
        if unsafe { command(DRIVER_NUM, GET_BYTES, buffer.len(), 0) } != 0 {
            // Unable to start the fetch, but we've already allow()-ed the
            // buffer to the kernel. Try to get it back.
            return match unsafe { allow_ptr(DRIVER_NUM, BUFFER_NUM, null_mut(), 0) } {
                0 => Err((FetchError::FAIL, Some(buffer))),
                _ => Err((FetchError::FAIL, None)),
            }
        }
        self.buffer_data.set(buffer.as_mut_ptr());
        self.buffer_len.set(buffer.len());
        return Ok(());
    }
}

/// Error type for the RNG driver.
// TODO: These are just the kernel-provided error types. For low-level drivers
// that "have no failure modes of their own", should we just have a common error
// type?
pub enum FetchError {
    FAIL = -1,        // Internal failure
    EBUSY = -2,       // A fetch is ongoing.
    ENODEVICE = -11,  // The kernel does not have the RNG capsule.
}

// We don't use crate::syscalls::subscribe as that requires a unique reference
// and we need subscribe to work with a shared reference. This is the callback
// we use instead.
unsafe extern fn callback<F: Forwarder<Option<Buffer>>>(_: usize, _: usize, _: usize, rng: usize) {
    let rng = &*(rng as *const Rng<F>);
    if allow_ptr(DRIVER_NUM, BUFFER_NUM, null_mut(), 0) != 0 {
        // Failed to un-allow the buffer. We leave the buffer values set in Rng,
        // which puts it into a "poisoned" state where it will return BUSY
        // forever.
        rng.forwarder.invoke_callback(None);
    }
    rng.forwarder.invoke_callback(Some(
        core::slice::from_raw_parts_mut(rng.buffer_data.replace(null_mut()), rng.buffer_len.get())
    ));
}
