//! Periodic timer. Exposes futures to wait for the next timer expiration.

use core::cell::Cell;
use crate::syscalls::{command, subscribe};
use crate::tock_static::TockStatic;

const ALARM: usize = 0;

const GET_FREQ: usize = 1;
const READ: usize = 2;
const SET_ALARM: usize = 4;

const ALARM_NOTIFICATIONS: usize = 0;

static PERIOD: TockStatic<Cell<usize>> = TockStatic::new(Cell::new(0));

const PERIOD_MS: u64 = 200;

// Future currently waiting on the alarm.
static WAKER: TockStatic<Cell<Option<core::task::Waker>>> = TockStatic::new(Cell::new(None));

// The current time. This is only approximately maintained for the purpose of
// this size test; in reality this is more complex to maintain.
static CUR_TIME: TockStatic<Cell<u64>> = TockStatic::new(Cell::new(0));

pub fn init() {
    PERIOD.set((PERIOD_MS * command(ALARM, GET_FREQ, 0, 0) as u64 / 1000) as usize);
    subscribe(ALARM, ALARM_NOTIFICATIONS, interrupt, &());
    set_delay();
}

pub fn wait() -> AlarmFuture {
    AlarmFuture {
        target: CUR_TIME.get() + PERIOD.get() as u64,
    }
}

pub struct AlarmFuture {
    // Target time for this alarm.
    target: u64,
}

impl core::future::Future for AlarmFuture {
    type Output = ();

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<()> {
        if CUR_TIME.get() >= self.target {
            return core::task::Poll::Ready(());
        }
        WAKER.set(Some(cx.waker().clone()));
        core::task::Poll::Pending
    }
}

extern "C" fn interrupt(_: usize, _: usize, _: usize, _: &()) {
    CUR_TIME.set(CUR_TIME.get() + PERIOD.get() as u64);
    set_delay();
    if let Some(waker) = WAKER.take() {
        waker.wake();
    }
}

// Creates an alarm period_ms in the future. If that time has passed, retries
// wiht a later time.
fn set_delay() {
    loop {
        let cur_tics = crate::syscalls::command(ALARM, READ, 0, 0);
        let target_time = cur_tics.wrapping_add(PERIOD.get());
        crate::syscalls::command(ALARM, SET_ALARM, target_time, 0);
        let after_time = crate::syscalls::command(ALARM, READ, 0, 0);
        if target_time.wrapping_sub(after_time) <= PERIOD.get() {
            // Successful set
            return
        }
    }
}
