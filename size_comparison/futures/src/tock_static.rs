/// TockStatic allows non-Sync objects to be used in `static` declarations. This
/// is unsafe in general Rust, but safe in the context of Tock applications as
/// Tock applications are always single-threaded.

#[repr(transparent)]
pub struct TockStatic<T> {
    value: T,
}

impl<T> TockStatic<T> {
    pub const fn new(value: T) -> TockStatic<T> {
        TockStatic { value }
    }
}

unsafe impl<T> Sync for TockStatic<T> {}

impl<T> core::ops::Deref for TockStatic<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}
