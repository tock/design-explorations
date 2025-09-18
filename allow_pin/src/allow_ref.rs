//! A design based around the `allow::Ref` type, which is a type that can point
//! to both 'static and stack-based buffers. `allow::Ref` itself is type (and
//! lifetime)-erased to avoid monomorphization bloat in APIs. However, it is
//! limited to only RO Allows or only RW Allows (because only allow::Ref<Ro> can
//! be created from a `&'static [u8]`).

pub type ErrorCode = u32;

pub mod allow {
    use core::ptr::null_mut;
    use crate::*;

    /// A reference to something that can be shared with the kernel via the
    /// Read-Only Allow system call (and if P is Rw, read-write Allow).
    pub struct Ref<P: Permissions, T: Allowable> {
        // The lifetime is erased to 'static here. There is no correct lifetime to
        // write when an AllowRef is embedded inside a Buffer.
        buffer: P::BufRef<'static, T>,
        // APIs need to be able to retrieve mutable references to the buffer
        // (for RW Allow), which requires that they have an exclusive reference
        // to the Ref. However, giving an &mut reference to the Ref would allow
        // the buffer to replace the Ref and forget it, which would prevent the
        // unallow from occurring before the buffer is dropped. To prevent that,
        // we pin the Ref as well.
        _pinned: PhantomPinned,
        share_info: Option<ShareInfo>,
    }

    impl<P: Permissions, T: Allowable> Ref<P, T> {
        fn unshare_if_shared(self: Pin<&mut Self>) {
            let this = unsafe { Pin::into_inner_unchecked(self) };
            let Some(ref info) = this.share_info else { return };
            unsafe {
                dynamic_allow(info.driver_num, info.buffer_num, null_mut(), 0, info.allow_type);
            }
            this.share_info = None;
        }
    }

    struct ShareInfo {
        allow_type: DynamicType,
        driver_num: u32,
        buffer_num: u32,
    }

    /// Trait representing an allowable type.
    pub trait Allowable: FromBytes + IntoBytes + 'static {}
    impl<T: FromBytes + IntoBytes + ?Sized + 'static> Allowable for T {}

    /// A type that represents whether a `Ref` has write access to a buffer.
    pub trait Permissions: sealed::Sealed {
        type BufRef<'b, T: Allowable>;
        // Only exists within the context of this design exploration; would not
        // exist in libtock-rs (because Permissions replaces StaticType
        // entirely).
        type Static: StaticType;
    }

    // Permissions implementations.
    pub enum Ro {}
    impl sealed::Sealed for Ro {}
    impl Permissions for Ro {
        type BufRef<'b, T: Allowable> = &'b T;
        type Static = StaticRo;
    }
    pub enum Rw {}
    impl sealed::Sealed for Rw {}
    impl Permissions for Rw {
        type BufRef<'b, T: Allowable> = &'b mut T;
        type Static = StaticRw;
    }

    mod sealed {
        pub trait Sealed {}
    }
}
