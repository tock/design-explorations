//! An allow buffer that tracks RO vs RW at runtime but which has a const ID.

use crate::*;

pub use crate::DynamicType;
pub type ErrorCode = u32;

pub struct Buffer<B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32> {
    _pinned: PhantomPinned,
    shared: Option<DynamicType>,
    buffer: B,
}

impl<B: Default + FromBytes + IntoBytes, const DRIVER_NUM: u32, const BUFFER_NUM: u32> Default
    for Buffer<B, DRIVER_NUM, BUFFER_NUM>
{
    fn default() -> Buffer<B, DRIVER_NUM, BUFFER_NUM> {
        Buffer {
            _pinned: PhantomPinned,
            shared: None,
            buffer: Default::default(),
        }
    }
}

impl<B: FromBytes + IntoBytes, const DRIVER_NUM: u32, const BUFFER_NUM: u32> From<B>
    for Buffer<B, DRIVER_NUM, BUFFER_NUM>
{
    fn from(buffer: B) -> Buffer<B, DRIVER_NUM, BUFFER_NUM> {
        Buffer {
            _pinned: PhantomPinned,
            shared: None,
            buffer,
        }
    }
}

// Possible surprising semantics: A Buffer that is created but never allowed
// will still clear its allow ID on drop!
impl<B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32> Drop
    for Buffer<B, DRIVER_NUM, BUFFER_NUM>
{
    fn drop(&mut self) {
        if let Some(p) = self.shared {
            unshare(DRIVER_NUM, BUFFER_NUM, p);
        }
    }
}

impl<B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32>
    Buffer<B, DRIVER_NUM, BUFFER_NUM>
{
    pub fn allow(self: Pin<&mut Self>, allow_type: DynamicType) -> Result<(), ErrorCode> {
        if self.shared.is_some() {
            return Err(3);
        }
        let this = unsafe { Pin::into_inner_unchecked(self) };
        unsafe { allow_inner(DRIVER_NUM, BUFFER_NUM, &mut this.buffer, allow_type) }?;
        this.shared = Some(allow_type);
        Ok(())
    }

    // Actually want separate unallow functions (which unshares + returns
    // buffer) and buffer functions (which checks it's unshared then returns the
    // buffer).
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
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            if let Some(p) = this.shared {
                unshare(DRIVER_NUM, BUFFER_NUM, p);
            }
            this.shared = None;
            &mut this.buffer
        }
    }

    pub fn share_status(self: Pin<&Self>) -> Option<DynamicType> {
        self.shared
    }

    pub fn replace_with<OB: FromBytes + IntoBytes + ?Sized>(
        self: Pin<&mut Self>,
        other: Pin<&mut Buffer<OB, DRIVER_NUM, BUFFER_NUM>>,
    ) -> (&mut B, Result<(), ErrorCode>) {
        let this = unsafe { Pin::into_inner_unchecked(self) };
        let other = unsafe { Pin::into_inner_unchecked(other) };
        let result = if let Some(allow_type) = this.shared {
            let _ = unsafe { allow_inner(DRIVER_NUM, BUFFER_NUM, &mut this.buffer, allow_type) };
            this.shared = None;
            other.shared = Some(allow_type);
            Ok(())
        } else {
            Err(6)
        };
        (&mut this.buffer, result)
    }
}

unsafe fn allow_inner<B: FromBytes + IntoBytes + ?Sized>(
    driver_num: u32,
    buffer_num: u32,
    buffer: &mut B,
    allow_type: DynamicType,
) -> Result<(), ErrorCode> {
    let (variant, r1, _, _) = unsafe {
        dynamic_allow(
            driver_num,
            buffer_num,
            buffer as *mut B as *mut u8,
            size_of_val(buffer),
            allow_type,
        )
    };
    if variant == 2 {
        return Err(r1.addr() as u32);
    }
    Ok(())
}

/// No error handling is needed because if (driver_num, buffer_num) is not
/// valid, then the buffer could not have been shared in the first place.
fn unshare(driver_num: u32, buffer_num: u32, allow_type: DynamicType) {
    unsafe {
        dynamic_allow(driver_num, buffer_num, null_mut(), 0, allow_type);
    }
}
