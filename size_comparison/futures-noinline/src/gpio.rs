//! Basic GPIO driver that only exposes what we need for this test.

use core::cell::Cell;
use crate::tock_static::TockStatic;

const DRIVER_NUM: usize = 4;

const ENABLE_OUTPUT: usize = 1;
const SET_PIN: usize = 2;
const CLEAR_PIN: usize = 3;
const ENABLE_INPUT: usize = 5;
const READ_PIN: usize = 6;
const CONFIG_INTERRUPTS: usize = 7;

const LED_PIN: usize = 0;
const BUTTON_PIN: usize = 1;

const EITHER_EDGE: usize = 0;
const PIN_CALLBACK: usize = 0;

static BUTTON_VALUE: TockStatic<Cell<bool>> = TockStatic::new(Cell::new(false));
static WAKER: TockStatic<Cell<Option<core::task::Waker>>> = TockStatic::new(Cell::new(None));

pub fn start() {
    crate::syscalls::command(DRIVER_NUM, ENABLE_OUTPUT, LED_PIN, 0);
    crate::syscalls::command(DRIVER_NUM, ENABLE_INPUT, BUTTON_PIN, 0);
    crate::syscalls::subscribe(DRIVER_NUM, PIN_CALLBACK, interrupt, &());
    crate::syscalls::command(DRIVER_NUM, CONFIG_INTERRUPTS, BUTTON_PIN, EITHER_EDGE);
}

// Precondition: start() already called.
pub fn read_button() -> bool {
    crate::syscalls::command(DRIVER_NUM, READ_PIN, BUTTON_PIN, 0) == 1
}

// Precondition: start() already called.
pub fn update(enable_light: bool) {
    let command = if enable_light { SET_PIN } else { CLEAR_PIN };
    crate::syscalls::command(DRIVER_NUM, command, LED_PIN, 0);
}

// Wait until the button state changes.
pub fn wait_button() -> ButtonFuture {
    ButtonFuture {
        need_value: !BUTTON_VALUE.get()
    }
}

pub struct ButtonFuture {
    need_value: bool,  // The button value we need to resolve.
}

impl core::future::Future for ButtonFuture {
    type Output = bool;

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<bool> {
        if BUTTON_VALUE.get() == self.need_value {
            return core::task::Poll::Ready(BUTTON_VALUE.get());
        }
        WAKER.set(Some(cx.waker().clone()));
        core::task::Poll::Pending
    }
}

extern "C" fn interrupt(_: usize, value: usize, _: usize, _: &()) {
    BUTTON_VALUE.set(value != 0);
    if let Some(waker) = WAKER.take() {
        waker.wake();
    }
}
