//! Periodically calls crate::app::App.alarm_fired();

use core::cell::Cell;
use crate::tock_static::TockStatic;

const ALARM: usize = 0;

const GET_FREQ: usize = 1;
const READ: usize = 2;
const SET_ALARM: usize = 4;

const ALARM_NOTIFICATIONS: usize = 0;

static FREQ: TockStatic<Cell<usize>> = TockStatic::new(Cell::new(0));
static PERIOD: TockStatic<Cell<usize>> = TockStatic::new(Cell::new(0));

const PERIOD_MS: usize = 200;

pub fn init() {
    FREQ.set(crate::syscalls::command(ALARM, GET_FREQ, 0, 0));
    crate::syscalls::subscribe(ALARM, ALARM_NOTIFICATIONS, interrupt, None);
}

// Start calling crate::app::APP.alarm_fired() periodically.
pub fn start(_: usize) {
    PERIOD.set(PERIOD_MS);
    set_delay();
}

// Creates an alarm PERIOD_MS in the future. If that time has passed, retries
// wiht a later time.
fn set_delay() {
    let delay_tics = (PERIOD_MS as u64 * FREQ.get() as u64 / 1000) as usize;
    loop {
        let cur_tics = crate::syscalls::command(ALARM, READ, 0, 0);
        let target_time = cur_tics.wrapping_add(delay_tics);
        crate::syscalls::command(ALARM, SET_ALARM, target_time, 0);
        let after_time = crate::syscalls::command(ALARM, READ, 0, 0);
        if target_time.wrapping_sub(after_time) <= delay_tics {
            // Successful set
            return
        }
    }
}

extern "C" fn interrupt(_: usize, _: usize, _: usize, _: Option<&()>) {
    set_delay();
    crate::app::APP.alarm_fired();
}
