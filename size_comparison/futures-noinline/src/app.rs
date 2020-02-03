//! Futures-based application.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use crate::tock_static::TockStatic;

pub static APP: TockStatic<App> = TockStatic::new(App::new());

pub struct App {
    poll_alarm: core::cell::Cell<bool>,
    poll_button: core::cell::Cell<bool>,
    task: crate::task::Task<AppFuture>,
    vtable: RawWakerVTable,
    waker: core::cell::Cell<Option<Waker>>,
}

impl App {
    pub const fn new() -> App {
        App {
            poll_alarm: core::cell::Cell::new(true),
            poll_button: core::cell::Cell::new(true),
            task: crate::task::Task::new(),
            vtable: RawWakerVTable::new(waker_clone, waker_wake, waker_wake, waker_drop),
            waker: core::cell::Cell::new(None),
        }
    }

    pub fn start(&'static self) {
        let _ = self.task.spawn(AppFuture::new());
    }
}

// RawWakerVTable clone entry.
fn waker_clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &APP.vtable)
}

// RawWakerVTable entry for wake and wake_by_ref.
fn waker_wake(event: *const ()) {
    unsafe {
        (*(event as *const core::cell::Cell<bool>)).set(true);
    }
    if let Some(waker) = APP.waker.take() {
        waker.wake();
    }
}

// All resources are static so this is a no-op.
fn waker_drop(_: *const ()) {}

// AppFuture should probably be a future combinator, but it's not obvious to me
// how it should be structured so I wrote it out by hand.
struct AppFuture {
    alarm: crate::alarm::AlarmFuture,
    button: crate::gpio::ButtonFuture,
    light: bool,
}

impl AppFuture {
    pub fn new() -> AppFuture {
        crate::gpio::update(false);
        AppFuture {
            alarm: crate::alarm::wait(),
            button: crate::gpio::wait_button(),
            light: false,
        }
    }

    #[inline(never)]
    fn alarm_fired(&mut self) {
        if crate::gpio::read_button() { return; }
        self.light = !self.light;
        crate::gpio::update(self.light);
    }

    #[inline(never)]
    fn button_event(&mut self) {
        if crate::gpio::read_button() {
            crate::gpio::update(false);
            self.light = false;
        } else {
            crate::gpio::update(true);
            self.light = true;
        }
    }
}

impl Future for AppFuture {
    type Output = Empty;

    #[inline(never)]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Empty> {
        APP.waker.set(Some(cx.waker().clone()));
        if APP.poll_alarm.take() {
            let ready = unsafe { Pin::new_unchecked(&mut self.alarm) }.poll(
                &mut Context::from_waker(&unsafe {
                    Waker::from_raw(RawWaker::new(
                        &APP.poll_alarm as *const core::cell::Cell<bool> as *const (),
                        &APP.vtable))
            })).is_ready();
            if ready {
                self.alarm_fired();
                self.alarm = crate::alarm::wait();
            }
        }
        if APP.poll_button.take() {
            let ready = unsafe { Pin::new_unchecked(&mut self.button) }.poll(
                &mut Context::from_waker(&unsafe {
                    Waker::from_raw(RawWaker::new(
                        &APP.poll_button as *const core::cell::Cell<bool> as *const (),
                        &APP.vtable))
            })).is_ready();
            if ready {
                self.button_event();
                self.button = crate::gpio::wait_button();
            }
        }
        Poll::Pending
    }
}

enum Empty {}
