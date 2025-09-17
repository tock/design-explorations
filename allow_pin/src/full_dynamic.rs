//! An Allow buffer that tracks whether the buffer is allowed, the allow type,
//! and the allow ID at runtime.

use crate::*;
use core::mem::size_of_val;

pub use crate::DynamicType;
pub type ErrorCode = u32;

pub struct Buffer<B: FromBytes + IntoBytes + ?Sized> {
    _pinned: PhantomPinned,
    shared: Option<ShareInfo>,
    buffer: B,
}

struct ShareInfo {
    allow_type: DynamicType,
    driver_num: u32,
    buffer_num: u32,
}

impl<B: Default + FromBytes + IntoBytes> Default for Buffer<B> {
    fn default() -> Buffer<B> {
        Buffer {
            _pinned: PhantomPinned,
            shared: None,
            buffer: Default::default(),
        }
    }
}

impl<B: FromBytes + IntoBytes> From<B> for Buffer<B> {
    fn from(buffer: B) -> Buffer<B> {
        Buffer {
            _pinned: PhantomPinned,
            shared: None,
            buffer,
        }
    }
}

impl<B: FromBytes + IntoBytes + ?Sized> Drop for Buffer<B> {
    fn drop(&mut self) {
        unshare_if_shared(&mut self.shared);
    }
}

impl<B: FromBytes + IntoBytes + ?Sized> Buffer<B> {
    // TODO: Try pulling the interior out into its own function that is not
    // generic.
    pub fn allow(
        self: Pin<&mut Self>,
        allow_type: DynamicType,
        driver_num: u32,
        buffer_num: u32,
    ) -> Result<(), ErrorCode> {
        if self.shared.is_some() {
            return Err(3);
        }
        let this = unsafe { Pin::into_inner_unchecked(self) };
        let (variant, r1, _, _) = unsafe {
            dynamic_allow(
                driver_num,
                buffer_num,
                &mut this.buffer as *mut _ as *mut u8,
                size_of_val(&this.buffer),
                allow_type,
            )
        };
        if variant == 2 {
            return Err(r1.addr() as u32);
        }
        this.shared = Some(ShareInfo {
            allow_type,
            driver_num,
            buffer_num,
        });
        Ok(())
    }

    pub fn buffer(self: Pin<&Self>) -> Option<&B> {
        if self.shared.is_some() {
            return None;
        }
        Some(&self.get_ref().buffer)
    }

    pub fn buffer_mut(self: Pin<&mut Self>) -> Option<&mut B> {
        if self.shared.is_some() {
            return None;
        }
        Some(&mut unsafe { Pin::into_inner_unchecked(self) }.buffer)
    }

    pub fn unallow(self: Pin<&mut Self>) -> &mut B {
        let this = unsafe { Pin::into_inner_unchecked(self) };
        unshare_if_shared(&mut this.shared);
        &mut this.buffer
    }
}

pub struct StaticBuffer<B: FromBytes + IntoBytes + 'static> {
    _pinned: PhantomPinned,
    buffer_ref: Option<&'static B>,
    // Reuse the `ShareInfo` struct to save memory
    shared: Option<ShareInfo>,
}

impl<B: FromBytes + IntoBytes + 'static> From<&'static B> for StaticBuffer<B> {
    fn from(buffer: &'static B) -> Self {
        Self {
            _pinned: Default::default(),
            buffer_ref: Some(buffer),
            shared: None,
        }
    }
}

impl<B: FromBytes + IntoBytes> Default for StaticBuffer<B> {
    fn default() -> Self {
        Self {
            _pinned: Default::default(),
            buffer_ref: None,
            shared: None,
        }
    }
}

impl<B: FromBytes + IntoBytes> Drop for StaticBuffer<B> {
    fn drop(&mut self) {
        unshare_if_shared(&mut self.shared);
    }
}

impl<B: FromBytes + IntoBytes> StaticBuffer<B> {
    pub fn allow(self: Pin<&mut Self>, driver_num: u32, buffer_num: u32) -> Result<(), ErrorCode> {
        if self.shared.is_some() {
            return Err(3);
        }
        if self.buffer_ref.is_none() {
            return Err(9);
        }

        let this = unsafe { Pin::into_inner_unchecked(self) };

        // SAFETY: `buffer_ref` previously checked to be `Some`
        let (variant, r1, _, _) = unsafe {
            dynamic_allow(
                driver_num,
                buffer_num,
                &mut this.buffer_ref.unwrap_unchecked() as *mut _ as *mut u8,
                size_of_val(this.buffer_ref.unwrap_unchecked()),
                DynamicType::Ro,
            )
        };
        if variant == 2 {
            return Err(r1.addr() as u32);
        }
        this.shared = Some(ShareInfo {
            allow_type: DynamicType::Ro,
            driver_num,
            buffer_num,
        });
        Ok(())
    }

    pub fn buffer(self: Pin<&Self>) -> Option<&B> {
        if self.shared.is_some() {
            return None;
        }
        self.get_ref().buffer_ref
    }

    pub fn unallow(self: Pin<&mut Self>) -> Option<&B> {
        let this = unsafe { Pin::into_inner_unchecked(self) };

        unshare_if_shared(&mut this.shared);
        this.buffer_ref
    }
}

// TODO: I separated this out from unshare() because I thought this should be
// inlined and unshare() should not, yet #[inline(never)] seems to have a
// positive impact here? Unsure why.
#[inline(never)]
fn unshare_if_shared(shared: &mut Option<ShareInfo>) {
    if let Some(info) = shared {
        unshare(&info);
        *shared = None;
    }
}

fn unshare(info: &ShareInfo) {
    unsafe {
        dynamic_allow(
            info.driver_num,
            info.buffer_num,
            null_mut(),
            0,
            info.allow_type,
        );
    }
}
