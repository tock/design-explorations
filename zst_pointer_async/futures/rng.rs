//! RNG driver. Fills user-provided buffers with random bytes.

use core::cell::Cell;
use core::pin::Pin;
use core::task::{Context, Poll};
use crate::lw::async_util::{Forwarder, TockStatic};
use crate::lw::rng::{Buffer, FetchError, Rng};

pub struct RngFuture {
    _private: (),
}

impl core::future::Future for RngFuture {
    type Output = Result<&'static mut [u8], LostBufferError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        match STATE.take() {
            State::Idle | State::LostBuffer => Poll::Ready(Err(LostBufferError)),
            State::Busy => {
                STATE.set(State::Busy);
                Poll::Pending
            },
            State::Done(buffer) => {
                Poll::Ready(Ok(buffer))
            },
        }
    }
}

pub struct LostBufferError;

impl core::fmt::Debug for LostBufferError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str("LostBufferError")
    }
}

impl RngFuture {
    pub fn new(buffer: &'static mut [u8]) -> Result<RngFuture, StartError> {
        let state = STATE.take();
        if state != State::Idle {
            STATE.set(state);
            return Err(StartError::EBUSY);
        }
        match RNG.fetch(buffer) {
            Ok(_) => { STATE.set(State::Busy); Ok(RngFuture { _private: () }) },
            Err((FetchError::FAIL, _)) => Err(StartError::FAIL),
            Err((FetchError::EBUSY, _)) => Err(StartError::EBUSY),  // Shouldn't happen...
            Err((FetchError::ENODEVICE, _)) => Err(StartError::ENODEVICE),
        }
    }
}

pub enum StartError {
    FAIL = -1,
    EBUSY = -2,
    ENODEVICE = -11,
}

impl core::fmt::Debug for StartError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str("StartError::")?;
        match self {
            StartError::FAIL => f.write_str("FAIL"),
            StartError::EBUSY => f.write_str("EBUSY"),
            StartError::ENODEVICE => f.write_str("ENODEVICE"),
        }
    }
}

// Open question: Should the futures API depend on `alloc`? That would allow us
// to recover from things like "someone started fetching random bytes then
// core::mem::forgot the future".

// TODO: To be compatible with the general futures ecosystem (e.g.
// futures::stream::FuturesUnordered), we should trigger the Waker in
// invoke_callback. We don't currently even store it.

#[derive(PartialEq)]
enum State {
    Idle,
    Busy,
    LostBuffer,
    Done(&'static mut [u8]),
}

impl core::default::Default for State {
    fn default() -> State {
        State::Idle
    }
}

static RNG: TockStatic<Rng<RngForwarder>> = TockStatic::new(Rng::new(RngForwarder));
static STATE: TockStatic<Cell<State>> = TockStatic::new(Cell::new(State::Idle));

#[derive(Clone, Copy)]
struct RngForwarder;

impl Forwarder<Option<Buffer>> for RngForwarder {
    fn invoke_callback(self, response: Option<Buffer>) {
        if let Some(buffer) = response {
            STATE.set(State::Done(buffer));
        } else {
            STATE.set(State::LostBuffer);
        }
    }
}
