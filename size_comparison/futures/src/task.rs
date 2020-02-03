//! Task is a lightweight future executor for libtock-rs applications. It should
//! be made a static variable. It starts empty and should have the future loaded
//! into it.

use core::task::{Context, RawWaker, RawWakerVTable, Waker};

pub struct Task<F: core::future::Future + 'static> {
    future: crate::tock_static::TockStatic<core::cell::RefCell<Option<F>>>,
    pending_poll: crate::tock_static::TockStatic<core::cell::Cell<bool>>,
    vtable: RawWakerVTable,
}

impl<F: core::future::Future + 'static> Task<F> {
    pub const fn new() -> Self {
        Task {
            future: crate::tock_static::TockStatic::new(core::cell::RefCell::new(None)),
            pending_poll: crate::tock_static::TockStatic::new(core::cell::Cell::new(false)),
            vtable: RawWakerVTable::new(
                Self::waker_clone,
                Self::waker_wake,
                Self::waker_wake,
                waker_drop,
            ),
        }
    }

    // Move the given future into the executor and starts executing it. Drops
    // the existing future. We cannot spawn a new future over the
    // currently-executing future; this will return an error if that is
    // attempted.
    pub fn spawn(&'static self, future: F) -> Result<(), CurrentlyPolling> {
        let mut borrow = match self.future.try_borrow_mut() {
            Ok(borrow) => borrow,
            Err(_) => return Err(CurrentlyPolling),
        };
        *borrow = Some(future);
        self.poll_future();
        Ok(())
    }

    // Polls the future in this task, if present.
    fn poll_future(&'static self) {
        let mut borrow = match self.future.try_borrow_mut() {
            Ok(borrow) => borrow,
            Err(_) => {
                // We were called from this task's future's poll() function!
                // Kinda weird, but rather than recursing back into poll() we'll
                // just let the outer invocation loop again.
                self.pending_poll.set(true);
                return;
            },
        };
        loop {
            let future = match &mut *borrow {
                None => return,
                Some(future) => future,
            };
            // We set this before polling because it may be overwritten if the
            // future calls back into poll_future().
            self.pending_poll.set(false);
            let waker = unsafe {
                Waker::from_raw(RawWaker::new(self as *const Self as *const (), &self.vtable))
            };
            let mut context = Context::from_waker(&waker);
            if unsafe{core::pin::Pin::new_unchecked(future)}.poll(&mut context).is_ready() {
                self.pending_poll.set(false);
            }
            if !self.pending_poll.get() {
                return;
            }
        }
    }

    // RawWakerVTable entries.
    fn waker_clone(this: *const ()) -> RawWaker {
        RawWaker::new(this, unsafe { &(*(this as *const Self)).vtable })
    }

    fn waker_wake(this: *const ()) {
        unsafe {
            (*(this as *const Self)).poll_future()
        };
    }
}

fn waker_drop(_: *const ()) {
    // No need to do anything; the Task is the raw waker itself and is static.
}

// Error type used to indicate a future tried to replace itself by calling
// spawn() on its own Task.
pub struct CurrentlyPolling;
