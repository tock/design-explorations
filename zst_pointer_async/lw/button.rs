//! Interface to the "button" syscall API. The button API allows applications to
//! read the state of the buttons and to receive interrupts when the button
//! state changes.

// TODO: Drivers that expose a mix of a synchronous and an asynchronous API are
// somewhat painful to use purely synchronously, because they still demand a
// forwarder even if one is not necessary. The "generic arguments only on the
// impl" approach probably handles this better.

use crate::lw::async_util::Forwarder;
use crate::returncode_subset;
use crate::syscalls::{command, subscribe_ptr};

const DRIVER_NUM: usize = 3;
const NUM_BUTTONS: usize = 0;
const ENABLE_INTERRUPT: usize = 1;
const DISABLE_INTERRUPT: usize = 2;
const GET_STATE: usize = 3;
const BUTTON_EVENT: usize = 0;

#[derive(Clone, Copy, PartialEq)]
pub struct Event {
    pub index: usize,
    pub new_value: bool,
}

pub struct Driver<F: Forwarder<Event>> {
    forwarder: F,
}

impl<F: Forwarder<Event>> Driver<F> {
    pub const fn new(forwarder: F) -> Driver<F> {
        Driver { forwarder }
    }

    // TODO: Result<usize, CountError> takes 2 words but there's less than 1
    // word worth of information there. If this is a repeated pattern (which
    // seems likely), then we may want to create an abstraction that packs this
    // information into a single isize (perhaps even with some "impossible
    // value" information in there?).
    pub fn get_num_buttons(&self) -> Result<usize, CountError> {
        let result = unsafe { command(DRIVER_NUM, NUM_BUTTONS, 0, 0) };
        // The only error we *should* get is ENODEVICE. If we get any other
        // error, it seems reasonable to coerce that error into ENODEVICE to
        // avoid including FAIL in CountError and increasing its size.
        if result < 0 { return Err(CountError::ENODEVICE); }
        Ok(result as usize)
    }

    pub fn enable_interrupt(&self, index: usize) -> Result<(), InterruptError> {
        let mut result = unsafe {
            subscribe_ptr(DRIVER_NUM, BUTTON_EVENT, callback::<F> as *const _,
                          self as *const Self as usize)
        };
        if result == crate::result::SUCCESS {
            result = unsafe { command(DRIVER_NUM, ENABLE_INTERRUPT, index, 0) };
        }
        match result {
            crate::result::SUCCESS => Ok(()),
            crate::result::ENOMEM => Err(InterruptError::ENOMEM),
            crate::result::EINVAL => Err(InterruptError::EINVAL),
            crate::result::ENODEVICE => Err(InterruptError::ENODEVICE),
            _ => Err(InterruptError::FAIL),
        }
    }

    pub fn disable_interrupt(&self, index: usize) -> Result<(), InterruptError> {
        match unsafe { command(DRIVER_NUM, DISABLE_INTERRUPT, index, 0) } {
            crate::result::SUCCESS => Ok(()),
            crate::result::ENOMEM => Err(InterruptError::ENOMEM),
            crate::result::EINVAL => Err(InterruptError::EINVAL),
            crate::result::ENODEVICE => Err(InterruptError::ENODEVICE),
            _ => Err(InterruptError::FAIL),
        }
    }

    pub fn get_state(&self, index: usize) -> Result<bool, GetStateError> {
        match unsafe { command(DRIVER_NUM, GET_STATE, index, 0) } {
            0 => Ok(false),
            1 => Ok(true),
            // We coerce all unknown errors to ENODEVICE. ENODEVICE is the only
            // error that should occur, and if another occurs then treating the
            // system call driver as missing seems reasonable.
            _ => Err(GetStateError::ENODEVICE),
        }
    }
}

returncode_subset![ pub enum CountError { ENODEVICE } ];
returncode_subset![ pub enum InterruptError { FAIL, ENOMEM, EINVAL, ENODEVICE } ];
returncode_subset![ pub enum GetStateError { ENODEVICE } ];

// We don't use crate::syscalls::subscribe as that requires a unique reference
// and we need subscribe to work with a shared reference. This is the callback
// we use instead.
unsafe extern fn callback<F: Forwarder<Event>>(index: usize, pressed: usize, _: usize, driver: usize) {
    let driver = &*(driver as *const Driver<F>);
    // Convert `pressed` into a bool for use in the Event. Security note:
    // although `pressed` should always be 0 or 1, applications do not trust
    // capsules (such as the Button driver), so we must not invoke UB if
    // `pressed` takes another value. The lightest way to handle other events is
    // to drop them, so that's what we de.
    let new_value = match pressed {
        0 => false,
        1 => true,
        _ => return,
    };
    driver.forwarder.invoke_callback(Event { index, new_value });
}
