/// Custom future combinators.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct WaitFirst<L: Future, R: Future> {
    left: L,
    right: R,
}

impl<L: Future, R: Future> WaitFirst<L, R> {
    pub fn new(left: L, right: R) -> WaitFirst<L, R> {
        WaitFirst { left, right }
    }
}

impl<L: Future, R: Future> Future for WaitFirst<L, R> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let self_mut = unsafe { self.get_unchecked_mut() };
        if let Poll::Ready(_) = unsafe { Pin::new_unchecked(&mut self_mut.left ) }.poll(cx)  {
            return Poll::Ready(());
        }
        if let Poll::Ready(_) = unsafe { Pin::new_unchecked(&mut self_mut.right) }.poll(cx)  {
            return Poll::Ready(());
        }
        Poll::Pending
    }
}
