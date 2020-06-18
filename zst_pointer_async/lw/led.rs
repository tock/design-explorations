//! Driver for the LED capsule. Contains 2 interface levels: a write-only
//! interface that lets users set and toggle LEDs and a read-write interface
//! that tracks whether an LED is set.

const DRIVER_NUM: usize = 2;
const TURN_ON: usize = 1;
const TURN_OFF: usize = 2;
const TOGGLE: usize = 3;

pub trait LedIdx {
    const IDX: usize;
}

pub struct Led<I: LedIdx> {
    _phantom: core::marker::PhantomData<I>,
}

impl<I: LedIdx> Led<I> {
    pub const fn new() -> Led<I> {
        Led { _phantom: core::marker::PhantomData }
    }

    fn led_op(&self, op: usize) -> Result<(), Error> {
        use crate::syscalls::command;
        match unsafe { command(DRIVER_NUM, op, I::IDX, 0) } {
            0 => Ok(()),
            -6 => Err(Error::EINVAL),
            -11 => Err(Error::ENODEVICE),
            _ => Err(Error::FAIL),
        }
    }

    pub fn turn_on(&self) -> Result<(), Error> {
        self.led_op(TURN_ON)
    }

    pub fn turn_off(&self) -> Result<(), Error> {
        self.led_op(TURN_OFF)
    }

    pub fn toggle(&self) -> Result<(), Error> {
        self.led_op(TOGGLE)
    }
}

pub enum Error {
    FAIL = -1,
    EINVAL = -6,
    ENODEVICE = -11,
}

/// LED driver that tracks the current state of the LED.
pub struct TrackedLed<I: LedIdx> {
    led: Led<I>,
    state: core::cell::Cell<Option<bool>>,
}

impl<I: LedIdx> TrackedLed<I> {
    pub fn new() -> TrackedLed<I> {
        TrackedLed { led: Led::new(), state: Default::default() }
    }

    /// Returns the LED's state, if known. Returns None if we do not know
    /// whether the LED is currently on.
    pub fn get_state(&self) -> Option<bool> {
        self.state.get()
    }

    /// Turns on the LED.
    pub fn turn_on(&self) -> Result<(), Error> {
        self.state.set(Some(true));
        self.led.turn_on()
    }

    /// Turns off the LED.
    pub fn turn_off(&self) -> Result<(), Error> {
        self.state.set(Some(false));
        self.led.turn_off()
    }

    /// Toggles the LED.
    pub fn toggle(&self) -> Result<(), Error> {
        self.state.set(self.state.get().map(core::ops::Not::not));
        self.led.toggle()
    }
}
