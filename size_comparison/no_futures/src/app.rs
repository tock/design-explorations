//! Blink-plus-button-interrupt app.

use crate::tock_static::TockStatic;
pub static APP: TockStatic<App> = TockStatic::new(App::new());

const DELAY_MS: usize = 200;

pub struct App {
    button: core::cell::Cell<bool>,
    light: core::cell::Cell<bool>,
}

impl App {
    pub const fn new() -> App{
        App { button: core::cell::Cell::new(false),
              light: core::cell::Cell::new(false) }
    }

    // Precondition: GPIO and Alarm already initialized.
    pub fn start(&self) {
        crate::gpio::update(false);
        self.button.set(crate::gpio::read_button());
        crate::alarm::start(DELAY_MS);
    }

    // Should be called by the GPIO driver whenever the button changes state.
    pub fn button_change(&self, value: bool) {
        if value {
            // Disable the light
            crate::gpio::update(false);
            self.light.set(false);
        } else {
            // Enable the light and an alarm.
            crate::gpio::update(true);
            self.light.set(true);
        }
        self.button.set(value);
    }

    // Called by the alarm driver.
    pub fn alarm_fired(&self) {
        // Ignore the timer if the button is pressed.
        if self.button.get() { return; }
        self.light.set(!self.light.get());
        crate::gpio::update(self.light.get());
    }
}
