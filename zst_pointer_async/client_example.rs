//! Example client application for the ZST pointers API in the lw/ directory.
//! This implements the same app used in the ../size_comparison writeup. The app
//! blinks a light, except it turns the light off whenever a button is held.

// -----------------------------------------------------------------------------
// State machine graph definition.
// It is possible to reduce the amount of boilerplate here with some
// macro_rules! macros.
// -----------------------------------------------------------------------------

/// ButtonClientPtr directs button events from the button driver to the main App
/// struct.
#[derive(Clone, Copy)]
struct ButtonClientPtr;
impl lw::async_util::AsyncClientPtr<lw::button::Event> for ButtonClientPtr {
    fn callback(self, output: lw::button::Event) {
        APP.button_event(output);
    }
}
/// BUTTON_DRIVER is the concrete instance of the button driver. Its client is
/// APP.
static BUTTON_DRIVER: lw::button::Driver<ButtonClientPtr> =
    lw::button::Driver::new(ButtonClientPtr);

/// ClockClientPtr directs timer events from the Clock to the main App struct.
#[derive(Clone, Copy)]
struct ClockClientPtr;
impl lw::async_util::AsyncClientPtr<lw::time::AlarmFired> for ClockClientPtr {
    fn callback(self, _output: lw::time::AlarmFired) {
        APP.alarm_fired();
    }
}
/// CLOCK is the concrete instance of lw::timer::Clock. Its client is APP.
static CLOCK: lw::async_util::TockStatic<lw::time::Clock<ClockClientPtr>> =
    lw::async_util::TockStatic::new(lw::time::Clock::new(ClockClientPtr));

/// AppLed specifies the LED the app controls.
struct AppLed;
impl lw::led::LedIdx for AppLed {
    const IDX: usize = 0;
}
/// LED is the concrete lw::led::Led instance. Its client is APP.
static LED: lw::led::Led<AppLed> = lw::led::Led::new();

/// APP is the struct containing the main application logic. It knows how to
/// find its dependencies because it -- and its dependencies -- are all part of
/// the application, rather than libtock-rs.
static APP: App = App::new();

// -----------------------------------------------------------------------------
// End state machine graph definition.
// -----------------------------------------------------------------------------

const BUTTON_IDX: usize = 0;

struct App {
}

impl App {
    pub const fn new() -> App {
        App {
        }
    }

    pub fn init(&self) {
        let _ = BUTTON_DRIVER.enable_interrupt(BUTTON_IDX);
    }

    pub fn run(&self) {
        // Set the first alarm, then run the event loop.
        while CLOCK.set_alarm(CLOCK.get_time() + 1000).is_err() {}
        loop { libtock::syscalls::yieldk(); }
    }

    pub fn button_event(&self, event: lw::button::Event) {
        if event.index == BUTTON_IDX && event.new_value {
            // Button just pressed, turn off the LED.
            LED.turn_off();
        }
    }

    pub fn alarm_fired(&self) {
        if BUTTON_DRIVER.get_state(BUTTON_IDX) == Ok(false) {
            // Button is not pressed, toggle LED.
            LED.toggle();
        }
        // Set the timer to fire again in 1000 ticks.
        while CLOCK.set_alarm(CLOCK.get_time() + 1000).is_err() {}
    }
}

pub fn main() {
    APP.init();
    CLOCK.init();

    APP.run();
}
