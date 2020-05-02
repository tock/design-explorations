use crate::lw::async_util::{Client, DynClient, Forwarder, TockStatic};
use crate::lw::sync_cell::SyncCell;
use crate::lw::time::{AlarmClock, AlarmFired, Clock, InPast};

pub static MUX: Mux = Mux {
    head: SyncCell::new(None),
};

pub struct Mux {
    head: SyncCell<Option<&'static MuxClient>>,
}

pub struct MuxClient {
    dyn_client: TockStatic<DynClient<'static, AlarmFired>>,
    next: SyncCell<Option<&'static MuxClient>>,
    setpoint: SyncCell<Option<u64>>,
}

impl MuxClient {
    pub const fn new<C: Client<AlarmFired>>(client: &'static C) -> MuxClient {
        MuxClient {
            dyn_client: TockStatic::new(DynClient::new(client)),
            next: SyncCell::new(None),
            setpoint: SyncCell::new(None),
        }
    }
}

impl AlarmClock for MuxClient {
    fn get_time(&self) -> u64 {
        CLOCK.get_time()
    }

    fn get_alarm(&self) -> u64 {
        self.setpoint.get().unwrap_or(u64::max_value())
    }

    fn set_alarm(&self, time: u64) -> Result<(), InPast> {
        if CLOCK.get_alarm() > time {
            // TODO: Implementing this correctly requires a deferred call
            // mechanism. If CLOCK.set_alarm returns Err(InPast), we not only
            // need to return InPast, we may also need to run callbacks and we
            // need to reset the existing alarm in CLOCK.

            // TODO: Perhaps it should've been a deferred-call-based mechanism
            // from the start? Maybe the APIs shouldn't be the same?
            return CLOCK.set_alarm(time);
        }

        self.setpoint.set(Some(time));
        Ok(())
    }
}

pub static CLOCK: TockStatic<Clock<MuxForwarder>> = TockStatic::new(Clock::new(MuxForwarder));

#[derive(Clone, Copy)]
pub struct MuxForwarder;

impl Forwarder<AlarmFired> for MuxForwarder {
    fn invoke_callback(self, _response: AlarmFired) {
        loop {
            let time = CLOCK.get_time();
            let mut cur_muxclient = MUX.head.get();
            while let Some(muxclient) = cur_muxclient {
                if let Some(setpoint) = muxclient.setpoint.get() {
                    if setpoint <= time {
                        // Set the setpoint 0 in case it is overwritten by the
                        // callback.
                        muxclient.setpoint.set(None);
                        muxclient.dyn_client.callback(AlarmFired);
                    }
                }
                cur_muxclient = muxclient.next.get();
            }
            let mut next_alarm = u64::max_value();
            cur_muxclient = MUX.head.get();
            while let Some(muxclient) = cur_muxclient {
                if let Some(setpoint) = muxclient.setpoint.get() {
                    if setpoint < next_alarm { next_alarm = setpoint; }
                }
            }
            if next_alarm == u64::max_value() || CLOCK.set_alarm(next_alarm).is_ok() {
                return;
            }
        }
    }
}
