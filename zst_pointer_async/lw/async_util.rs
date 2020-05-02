//! Module containing async building blocks used by this lightweight libtock-rs
//! prototype.

use core::cell::{Cell, UnsafeCell};

/// A trait implemented by clients of asynchronous components. Has a callback
/// that receives a value of type T.
pub trait Client<T> {
    fn callback(&self, response: T);
}

/// A lighter-weight version of &dyn Client<T>. &dyn references internally
/// contain two pointers: the pointer to the object's data and a pointer to a
/// vtable. The vtable contains the type's size, alignment, destructor, and
/// callback pointer, and is therefore 4 words in size. DynClient contains two
/// words: the data pointer and the callback pointer.
pub struct DynClient<'a, T> {
    data: *const (),

    // Because we don't know the real type of client, we have to erase the type
    // of the data pointer. As far as I can tell, Rust does not *guarantee* that
    // two implementations of Client<T>::callback have the same ABI, even when
    // the T is the same in both cases. Instead, we point to a shim that has a
    // fixed ABI regardless of the underlying client, and hope that shim
    // optimizes away.
    callback: unsafe fn(*const (), T),

    _phantom: core::marker::PhantomData<&'a dyn Client<T>>,
}

impl<'a, T> DynClient<'a, T> {
    pub const fn new<C: Client<T>>(client: &'a C) -> DynClient<'a, T> {
        DynClient {
            data: client as *const C as *const (),
            callback: erased_call::<T, C>,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn callback(&self, response: T) {
        unsafe { (self.callback)(self.data, response); }
    }
}

unsafe fn erased_call<T, C: Client<T>>(data: *const (), response: T) {
    C::callback(&*(data as *const C), response)
}

/// A trait for "forwarders", which are type system shims that route callbacks
/// to the appropriate client. Asynchronous components are generally generic
/// over a forwarder; the forwarder provides them a way to route a callback to
/// the client that does not require the asynchronous component to store a
/// pointer to the client.
///
/// The forwarders are Copy and take `self` (rather than `&self`) so that if
/// they are implemented as a zero-sized type the self argument will have no
/// overhead.
pub trait Forwarder<T>: Copy {
    fn invoke_callback(self, response: T);
}

/// Container that wraps a global value and hands out `&'static mut` references
/// to it. The reference validity is checked at runtime. StaticMutCell should be
/// used in a normal `static` item, not a `static mut` item. It is only sound
/// with `libtock-rs`'s threading model: it assumes there are no other running
/// threads.
pub struct StaticMutCell<T> {
    value: UnsafeCell<T>,
    borrowed: Cell<bool>,
}

// Assumes single-threaded operation. We implement Sync so a StaticMutCell can
// be stored in a normal `static`, so that users see a safe interface.
unsafe impl<T> Sync for StaticMutCell<T> {}

impl<T> StaticMutCell<T> {
    pub const fn new(value: T) -> StaticMutCell<T> {
        StaticMutCell { value: UnsafeCell::new(value), borrowed: Cell::new(false) }
    }

    pub fn get(&'static self) -> Option<&'static mut T> {
        if self.borrowed.get() {
            return None;
        }
        self.borrowed.set(true);
        Some(unsafe { &mut *self.value.get() })
    }

    pub fn unborrow(&self, reference: &'static mut T) {
        // For safety, We need to make sure that `reference` points to *our* T,
        // rather than a different T. StaticMutCell is never a ZST (because
        // `borrowed` has positive size), so we cannot have two distinct
        // StaticMutCells at the same address. Therefore we can just compare
        // pointer values.
        if self.value.get() == reference as *mut T {
            self.borrowed.set(false);
        }
    }
}

/// TockStatic allows non-Sync objects to be used in `static` declarations. This
/// is unsafe in general Rust, but safe in the context of Tock applications as
/// Tock applications are always single-threaded.
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

// ClientList is essentially &[&dyn Client<T>], but zero-sized (to remove
// runtime overhead).
pub unsafe trait List<T> {
    const LEN: usize;
    fn get(&self) -> &'static [DynClient<'static, T>];
}

pub struct EmptyList<T> { _phantom: core::marker::PhantomData<T> }

impl<T> EmptyList<T> {
    pub const fn new() -> EmptyList<T> {
        EmptyList { _phantom: core::marker::PhantomData }
    }
}

unsafe impl<T> List<T> for EmptyList<T> {
    const LEN: usize = 0;

    fn get(&self) -> &'static [DynClient<'static, T>] {
        &[]
    }
}

#[repr(C)]
pub struct ClientList<T: 'static, L: List<T>> {
    dyn_client: DynClient<'static, T>,
    rest: L,
}

impl<T: 'static, L: List<T>> ClientList<T, L> {
    pub const fn new<C: Client<T>>(client: &'static C, rest: L) -> ClientList<T, L> {
        ClientList { dyn_client: DynClient::new(client), rest }
    }
}

unsafe impl<T: 'static, L: List<T>> List<T> for ClientList<T, L> {
    const LEN: usize = L::LEN + 1;

    fn get(&self) -> &'static [DynClient<'static, T>] {
        use core::slice::from_raw_parts;
        unsafe {
            from_raw_parts(self as *const Self as *const DynClient<T>, L::LEN + 1)
        }
    }
}
