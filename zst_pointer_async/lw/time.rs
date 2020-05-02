//! Lightweight Timer/Alarm interface.

// Most of Clock's complexity comes from un-wrapping the 32-bit counter exposed
// by the kernel into a 64-bit counter. To do this, we make sure to set alarms
// no longer than UPDATE_PERIOD apart, so that we can catch wrapping events.
//
// Note that we cannot set UPDATE_PERIOD to 2^32 or 2^32 - 1, as otherwise the
// following two sequences of events would be indistinguishable:
//   Alarm expires           get_time() is called
//   get_time() is called    Alarm expires
//   Alarm callback runs     Alarm callback runs
// In the first sequence, the get_time() call needs to handle the full
// wraparound, but the numbers returned by the kernel would match the second
// sequence. Therefore, we need all queued alarm callbacks to run less than
// 2^32 - UPDATE_PERIOD ticks after the alarm fires. We don't have a way to make
// that happen (as Tock is not a RTOS) so we simply hope such a large delay
// never happens.

use crate::lw::async_util::Forwarder;
use crate::syscalls::{command, subscribe_ptr};

const DRIVER_NUM: usize = 0;
const GET_TICKS: usize = 2;
const SET_ALARM: usize = 4;
const ALARM_FIRED: usize = 0;
const UPDATE_PERIOD: u32 = 1_000_000_000;

pub struct Clock<F: Forwarder<AlarmFired>> {
    // The time at which the client wants to be alerted.
    client_setpoint: core::cell::Cell<u64>,

    // Time as of init or the last kernel alarm. The lower 32 bits equal the
    // kernel counter value at that time.
    last_callback: core::cell::Cell<u64>,

    forwarder: F,
}

pub trait AlarmClock {
    fn get_time(&self) -> u64;
    fn get_alarm(&self) -> u64;
    fn set_alarm(&self, time: u64) -> Result<(), InPast>;
}

impl<F: Forwarder<AlarmFired>> Clock<F> {
    pub const fn new(forwarder: F) -> Clock<F> {
        Clock {
            client_setpoint: core::cell::Cell::new(0),
            last_callback: core::cell::Cell::new(0),
            forwarder
        }
    }

    // TODO: Figure out if we want to enforce init() before using the clock +
    // design the mechanism for doing so.
    pub fn init(&self) {
        unsafe {
            subscribe_ptr(DRIVER_NUM, ALARM_FIRED, callback::<F> as *const _,
                          self as *const Self as usize);
        }
        self.client_setpoint.set(u64::max_value());
        self.last_callback.set(unsafe { command(DRIVER_NUM, GET_TICKS, 0, 0) } as u64);
        let callback_ticks = (self.last_callback.get() as u32).wrapping_add(UPDATE_PERIOD);
        unsafe { command(DRIVER_NUM, SET_ALARM, callback_ticks as usize, 0) };
    }
}

impl<F: Forwarder<AlarmFired>> AlarmClock for Clock<F> {
    fn get_time(&self) -> u64 {
        calc_new_unwrapped(
            self.last_callback.get(),
            unsafe { command(DRIVER_NUM, GET_TICKS, 0, 0) } as u32,
        )
    }

    // Returns u64::max_value() if no alarm is set.
    fn get_alarm(&self) -> u64 {
        self.client_setpoint.get()
    }

    fn set_alarm(&self, time: u64) -> Result<(), InPast> {
        self.client_setpoint.set(time);
        if time >= self.last_callback.get() + UPDATE_PERIOD as u64 {
            return Ok(());
        }
        unsafe { command(DRIVER_NUM, SET_ALARM, time as usize, 0) };
        if self.get_time() >= time {
            let callback_ticks = (self.last_callback.get() as u32).wrapping_add(UPDATE_PERIOD);
            unsafe { command(DRIVER_NUM, SET_ALARM, callback_ticks as usize, 0) };
            self.client_setpoint.set(u64::max_value());
            return Err(InPast);
        }
        Ok(())
    }
}

pub struct AlarmFired;
pub struct InPast;

// Finds the next 64-bit time value that matches the provided 32-bit kernel time
// value.
fn calc_new_unwrapped(unwrapped: u64, ticks: u32) -> u64 {
    unwrapped + ticks.wrapping_sub(unwrapped as u32) as u64
}

// We don't use crate::syscalls::subscribe as that requires a unique reference
// and we need subscribe to work with a shared reference. This is the callback
// we use instead.
unsafe extern fn callback<F: Forwarder<AlarmFired>>(_: usize, expired: usize, _: usize, clock: usize) {
    use core::cmp::min;
    let clock = &*(clock as *const Clock<F>);
    clock.last_callback.set(calc_new_unwrapped(clock.last_callback.get(), expired as u32));
    loop {
        let tgt = min(clock.client_setpoint.get(), clock.last_callback.get() + UPDATE_PERIOD as u64);
        command(DRIVER_NUM, SET_ALARM, tgt as usize, 0);
        if clock.get_time() >= clock.client_setpoint.get() {
            clock.client_setpoint.set(u64::max_value());
            clock.forwarder.invoke_callback(AlarmFired);
            continue;
        }
        break;
    }
}
