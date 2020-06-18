//! Provides a Future executors for libtock-rs tasks.

use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// Runs the provided Future until it completes, then returns the result.
pub fn block_on<F: core::future::Future>(mut future: F) -> F::Output {
    use core::ptr;

    let waker = unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    loop {
        let future = unsafe { core::pin::Pin::new_unchecked(&mut future) };
        match future.poll(&mut context) {
            Poll::Pending => crate::syscalls::yieldk(),
            Poll::Ready(value) => return value,
        }
    }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

fn clone(data: *const ()) -> RawWaker { RawWaker::new(data, &VTABLE) }
fn noop(_: *const ()) {}
