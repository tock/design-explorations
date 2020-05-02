//! Futures-based interface to the "buttons" syscall API. The buttons API allows
//! applications to read whether or not buttons are pressed and to receive
//! notifications when a button's state changes.

// TODO: This is definitely an interface that would appreciate `alloc`. We
// currently make users statically allocate per-button data, but with `alloc`
// that could be dynamic.

use core::cell::Cell;
use core::convert::TryFrom;
use core::task::{Context, Poll};
use crate::lw::async_util::{Forwarder, TockStatic};
use crate::lw::button::{Driver, Event, GetStateError};
use crate::returncode_subset;

pub fn get_state(index: usize) -> Result<bool, GetStateError> {
    DRIVER.get_state(index)
}

/// Holds static data structures required to route Button events to the correct
/// future. Initializing ButtonFuture requires passing it a &'static Button.
pub struct Button {
    next: TockStatic<Cell<Option<&'static Button>>>,
    state: TockStatic<Cell<ButtonState>>,
}

impl Button {
    pub const fn new() -> Button {
        Button { next: TockStatic::new(Cell::new(None)),
                 state: TockStatic::new(Cell::new(ButtonState::Uninitialized)) }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ButtonState {
    Uninitialized,
    Idle,
    WaitingFor(Event),
    Fired,
}

/// A future that waits for a button to be pressed or released.
pub struct ButtonFuture {
    button: &'static Button,
}

impl ButtonFuture {
    /// Creates a new ButtonFuture that waits until the button state has the
    /// specified value.
    pub fn new(button: &'static Button, index: usize, new_value: bool) -> Result<ButtonFuture, StartError> {
        if button.state.get() == ButtonState::Uninitialized {
            button.next.set(BUTTON_LIST.replace(Some(button)));
        } else if button.state.get() != ButtonState::Idle {
            // This Button is currently in use.
            return Err(StartError::EBUSY);
        }
        DRIVER.enable_interrupt(index).map_err(
            |ie| TryFrom::try_from(ie as isize).unwrap_or(StartError::FAIL))?;
        button.state.set(ButtonState::WaitingFor(Event { index, new_value }));
        Ok(ButtonFuture { button })
    }
}

returncode_subset![ pub enum StartError { FAIL, EBUSY, ENOMEM, EINVAL, ENODEVICE } ];

impl core::future::Future for ButtonFuture {
    type Output = ();

    // TODO: This should store and invoke the Waker for compatibility with
    // external future combinators. We don't currently do so.
    fn poll(self: core::pin::Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        if let ButtonState::WaitingFor(_) = self.button.state.get() {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

static BUTTON_LIST: TockStatic<Cell<Option<&'static Button>>> = TockStatic::new(Cell::new(None));
static DRIVER: Driver<FutureForwarder> = Driver::new(FutureForwarder);

#[derive(Clone, Copy)]
struct FutureForwarder;

impl Forwarder<Event> for FutureForwarder {
    fn invoke_callback(self, response: Event) {
        let mut opt_button: Option<&'static Button> = BUTTON_LIST.get();
        while let Some(button) = opt_button {
            if let ButtonState::WaitingFor(event) = button.state.get() {
                if event != response { break; }
                button.state.set(ButtonState::Fired);
            }
            opt_button = button.next.get();
        }
    }
}
