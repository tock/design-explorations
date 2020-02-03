//! Basic GPIO driver that only exposes what we need for this test.

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

pub fn start() {
    crate::syscalls::command(DRIVER_NUM, ENABLE_OUTPUT, LED_PIN, 0);
    crate::syscalls::command(DRIVER_NUM, ENABLE_INPUT, BUTTON_PIN, 0);
    crate::syscalls::subscribe(DRIVER_NUM, PIN_CALLBACK, interrupt, None);
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

extern "C" fn interrupt(_: usize, value: usize, _: usize, _: Option<&()>) {
    crate::app::APP.button_change(value == 1);
}
