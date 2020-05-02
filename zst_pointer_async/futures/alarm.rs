/// Futures-based interface to wait for a given amount of time. Uses the
/// lightweight timer/alarm driver.

use core::task::{Context, Poll};
use crate::lw::time::{AlarmClock, AlarmFired};

pub struct AlarmClockClient;

impl crate::lw::async_util::Client<AlarmFired> for AlarmClockClient {
    fn callback(&self, _response: AlarmFired) {}
}

pub struct AlarmFuture<C: AlarmClock + 'static> {
    clock: &'static C,
    setpoint: u64,
}

impl<C: AlarmClock> AlarmFuture<C> {
    // Sets an alarm for `delay` ticks in the future.
    pub fn new(clock: &'static C, delay: u64) -> AlarmFuture<C> {
        AlarmFuture { clock, setpoint: delay + clock.get_time() }
    }
}

impl<C: AlarmClock> core::future::Future for AlarmFuture<C> {
    type Output = ();

    fn poll(self: core::pin::Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        let cur_alarm = self.clock.get_alarm();
        if cur_alarm > self.setpoint {
            if self.clock.set_alarm(self.setpoint).is_ok() {
                return Poll::Pending;
            }
            // I'm not sure whether ignoring this error is a bug. This logic
            // would probably change if we store a Waker.
            let _ = self.clock.set_alarm(cur_alarm);
            return Poll::Ready(());
        }
        Poll::Pending
    }
}

#[derive(Clone, Copy)]
pub struct FutureForwarder;

impl crate::lw::async_util::Forwarder<AlarmFired> for FutureForwarder {
    fn invoke_callback(self, _: AlarmFired) {
        // No-op. The setpoint has already been reset, and the futures that
        // expired will poll CLOCK to determine that they have expired.
        // TODO: This should store a Waker and poll it instead of assuming
        // futures will be polled.
    }
}
