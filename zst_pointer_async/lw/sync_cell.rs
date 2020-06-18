/// SyncCell exposes the a similar API to core::cell::Cell, but is Sync. In
/// single-threaded environments (such as the current implementation of
/// libtock-rs), a SyncCell has no overhead relative to a core::cell::Cell. In
/// multi-threaded environments, SyncCell can be implemented using a mutex (or
/// atomic types, if available).
// repr(transparent) is ommitted because it is not compatible with a mutex-based
// implementation.
pub struct SyncCell<T: ?Sized> {
    // This implementation is for single-threaded environments (i.e. the current
    // libtock).
    value: core::cell::Cell<T>,
}

unsafe impl<T: ?Sized> Sync for SyncCell<T> {}

impl<T> SyncCell<T> {
    pub const fn new(value: T) -> SyncCell<T> {
        SyncCell { value: core::cell::Cell::new(value) }
    }

    pub fn set(&self, val: T) {
        self.value.set(val);
    }

    pub fn swap(&self, other: &Self) {
        self.value.swap(&other.value);
    }

    pub fn replace(&self, val: T) -> T {
        self.value.replace(val)
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: Copy> SyncCell<T> {
    pub fn get(&self) -> T {
        self.value.get()
    }
}

impl<T: ?Sized> SyncCell<T> {
    pub const fn as_ptr(&self) -> *mut T {
        self.value.as_ptr()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    // from_mut() is omitted because it is not compatible with a multithreaded
    // runtime.
}

impl<T: Default> SyncCell<T> {
    pub fn take(&self) -> T {
        self.value.take()
    }
}

// as_slice_of_cells() is omitted because it is not compatible with a
// mutex-based implementation.

impl<T: Copy> Clone for SyncCell<T> {
    fn clone(&self) -> SyncCell<T> {
        SyncCell { value: self.value.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.value.clone_from(&source.value);
    }
}

// Debug skipped because it generates size bloat (TODO: introduce a
// lighter-weight alternative and implement that instead).

impl<T: Default> Default for SyncCell<T> {
    fn default() -> SyncCell<T> {
        SyncCell { value: Default::default() }
    }
}

// TODO: The rest of the trait implementations
