//! A lighter-weight alternative to Rust's virtual call abstractions.
//! `DynCall<Args, Return>` functions similarly to a
//! `&dyn Callable<Args, Return>`, but instead of being a (*data, *vtable) pair
//! it is a (*data, *function) pair. This removes one level of indirection, and
//! removes the entire vtable (which would be 4 words in size). The downside is
//! it only supports a single method, and cannot be used with owning types (i.e.
//! you cannot use it the same way as a &dyn reference in Box<&dyn Callable>).

use core::num::NonZeroUsize;

/// Represents a method on a type. This is intentionally similar to the Fn trait
/// (which is unstable).
pub trait Callable<Args, Return>: Sized {
    fn call(&self, args: Args) -> Return;

    /// Type-erased version of call, for use by DynCall. Safety: safe to call
    /// iff `data` can safely be casted to &Self for the underlying type.
    /// TODO: Verify this is deduplicated with call.
    unsafe fn erased_call(data: NonZeroUsize, args: Args) -> Return {
        Self::call(&*(data.get() as *const Self), args)
    }
}

/// Lighter-weight version of `&dyn Callable<Args, Return>` that inlines the
/// function pointer directly rather than indirecting through a vtable.
pub struct DynCall<Args, Return> {
    data: NonZeroUsize,
    function: unsafe fn(NonZeroUsize, Args) -> Return,
}

impl<Args, Return> DynCall<Args, Return> {
    /// Constructs a new dyncall pointing to the given callable object.
    pub fn new<T: Callable<Args, Return>>(object: &T) -> Self {
        DynCall { data: unsafe { NonZeroUsize::new_unchecked(object as *const T as usize) },
                  function: T::erased_call }
    }

    /// Call the pointed-to method on the pointed-to object.
    pub fn call(&self, args: Args) -> Return {
        unsafe { (self.function)(self.data, args) }
    }
}
