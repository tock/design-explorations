//! A maximally-static implementation: no dynamic RO/RW, no dynamic ID. Every
//! operation that requires the buffer to be unshared unconditionally unshares it.

use crate::*;
use core::ptr;

pub use crate::{StaticRo, StaticRw};

pub struct Buffer<
    P: StaticType,
    B: FromBytes + IntoBytes + ?Sized,
    const DRIVER_NUM: u32,
    const BUFFER_NUM: u32,
> {
    _perms: PhantomData<P>,
    _pinned: PhantomPinned,
    buffer: B,
}

impl<
    P: StaticType,
    B: Default + FromBytes + IntoBytes,
    const DRIVER_NUM: u32,
    const BUFFER_NUM: u32,
> Default for Buffer<P, B, DRIVER_NUM, BUFFER_NUM>
{
    fn default() -> Buffer<P, B, DRIVER_NUM, BUFFER_NUM> {
        Buffer {
            _perms: PhantomData,
            _pinned: PhantomPinned,
            buffer: Default::default(),
        }
    }
}

impl<P: StaticType, B: FromBytes + IntoBytes, const DRIVER_NUM: u32, const BUFFER_NUM: u32> From<B>
    for Buffer<P, B, DRIVER_NUM, BUFFER_NUM>
{
    fn from(buffer: B) -> Buffer<P, B, DRIVER_NUM, BUFFER_NUM> {
        Buffer {
            _perms: PhantomData,
            _pinned: PhantomPinned,
            buffer,
        }
    }
}

// Possible surprising semantics: A Buffer that is created but never allowed
// will still clear its allow ID on drop!
impl<P: StaticType, B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32>
    Drop for Buffer<P, B, DRIVER_NUM, BUFFER_NUM>
{
    fn drop(&mut self) {
        unshare::<P>(DRIVER_NUM, BUFFER_NUM);
    }
}

// Read-Only methods
impl<B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32>
    Buffer<StaticRo, B, DRIVER_NUM, BUFFER_NUM>
{
    pub fn allow_ro(self: Pin<&Self>) -> Result<(), ErrorCode> {
        unsafe {
            allow_inner::<StaticRo>(
                DRIVER_NUM,
                BUFFER_NUM,
                ptr::slice_from_raw_parts(
                    (&raw const self.buffer).cast(),
                    size_of_val(&self.buffer),
                ),
            )
        }
    }

    pub fn buffer(self: Pin<&Self>) -> &B {
        &self.get_ref().buffer
    }

    /// Allows `new`, un-allowing `self`. Returns a reference to `self`'s
    /// buffer.
    pub fn replace_with_ro<OB: FromBytes + IntoBytes + ?Sized>(
        self: Pin<&Self>,
        new: Pin<&Buffer<StaticRo, OB, DRIVER_NUM, BUFFER_NUM>>,
    ) -> (&B, Result<(), ErrorCode>) {
        (self.buffer(), new.allow_ro())
    }

    /// Allows `new`, un-allowing `self`. Returns a mutable reference to
    /// `self`'s buffer.
    pub fn replace_with_mut_ro<OB: FromBytes + IntoBytes + ?Sized>(
        self: Pin<&mut Self>,
        new: Pin<&Buffer<StaticRo, OB, DRIVER_NUM, BUFFER_NUM>>,
    ) -> (&mut B, Result<(), ErrorCode>) {
        (
            &mut unsafe { Pin::into_inner_unchecked(self) }.buffer,
            new.allow_ro(),
        )
    }
}

// Read-Write methods
impl<B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32>
    Buffer<StaticRw, B, DRIVER_NUM, BUFFER_NUM>
{
    pub fn buffer(self: Pin<&Self>) -> &B {
        unshare::<StaticRw>(DRIVER_NUM, BUFFER_NUM);
        &self.get_ref().buffer
    }
}

// Methods that exist in both Read-Only and Read-Write Allow.
impl<P: StaticType, B: FromBytes + IntoBytes + ?Sized, const DRIVER_NUM: u32, const BUFFER_NUM: u32>
    Buffer<P, B, DRIVER_NUM, BUFFER_NUM>
{
    pub fn allow(self: Pin<&mut Self>) -> Result<(), ErrorCode> {
        unsafe {
            let this = self.get_unchecked_mut();
            allow_inner::<P>(
                DRIVER_NUM,
                BUFFER_NUM,
                ptr::slice_from_raw_parts_mut(
                    (&raw mut this.buffer).cast(),
                    size_of_val(&this.buffer),
                ),
            )
        }
    }

    // Possible surprising semantics: Retrieving the buffer performs an unallow,
    // even if a different buffer is shared with this allow ID! (applies to
    // buffer() as well).
    pub fn buffer_mut(self: Pin<&mut Self>) -> &mut B {
        unshare::<P>(DRIVER_NUM, BUFFER_NUM);
        &mut unsafe { Pin::into_inner_unchecked(self) }.buffer
    }

    /// Allows `new`, un-allowing `self`. Returns a reference to `self`'s
    /// buffer.
    pub fn replace_with<OB: FromBytes + IntoBytes + ?Sized>(
        self: Pin<&Self>,
        new: Pin<&mut Buffer<P, OB, DRIVER_NUM, BUFFER_NUM>>,
    ) -> (&B, Result<(), ErrorCode>) {
        let result = new.allow();
        (&self.get_ref().buffer, result)
    }

    /// Allows `new`, un-allowing `self`. Returns a mutable reference to
    /// `self`'s buffer.
    pub fn replace_with_mut<OB: FromBytes + IntoBytes + ?Sized>(
        self: Pin<&mut Self>,
        new: Pin<&mut Buffer<P, OB, DRIVER_NUM, BUFFER_NUM>>,
    ) -> (&mut B, Result<(), ErrorCode>) {
        let result = new.allow();
        (
            &mut unsafe { Pin::into_inner_unchecked(self) }.buffer,
            result,
        )
    }
}

unsafe fn allow_inner<P: StaticType>(
    driver_num: u32,
    buffer_num: u32,
    buffer: *const [u8],
) -> Result<(), ErrorCode> {
    let (variant, r1, _, _) =
        unsafe { static_allow::<P>(driver_num, buffer_num, buffer as *mut _, buffer.len()) };
    if variant == 2 {
        return Err(r1.addr() as u32);
    }
    Ok(())
}

/// Performs an "unallow" call -- unshares the given buffer with the kernel.
/// Postcondition: no buffer will be shared with the kernel with ID
/// (driver_num, buffer_num).
/// No error handling is needed because if (driver_num, buffer_num) is not
/// valid, then the buffer could not have been shared in the first place.
fn unshare<P: StaticType>(driver_num: u32, buffer_num: u32) {
    unsafe {
        static_allow::<P>(driver_num, buffer_num, null_mut(), 0);
    }
}
