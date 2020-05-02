//! Provides facilities for generating error enums representing a subset of the
//! kernel's error codes (defined at
//! https://github.com/tock/tock/blob/master/kernel/src/returncode.rs). To make
//! conversions more efficient, they have the same numeric values as the
//! ReturnCode values. They have fewer possibilites to prevent unnecessary match
//! branches.

/// Generates an enum with the specified subset of ReturnCode's values. For
/// example:
/// ```
/// returncode_subset![ enum Error {
///     SUCCESS,
///     EOFF,
/// } ];
/// ```
/// expands to:
/// ```
/// enum Error {
///     SUCCESS = 0,
///     EOFF = -4,
/// };
/// ```
// TODO: Do we want to auto-implement TryFrom for the type? It's unclear whether
// most uses of these structs would want to use custom code for each case or
// whether there would be a significant number of drivers that would translate
// directly. This may also change if we add a 1-word ReturnCode type that packs
// a Result<positive isize, Error> into a single struct.
// TODO: Do we want to have conversions between different ReturnCode subsets? If
// so, how? Fallable at runtime? Enforced at compile-time (one trait per
// ReturnCode variant?).
// TODO: What is the backwards-compatibility story here? Can syscall APIs add
// new errors codes as they wish? If not, then yay. If yes, do we want to do
// something like #[non_exhaustive]? Do we want to force the userspace drivers
// to coerce errors to the errors they've stabilized?
#[macro_export]
macro_rules! returncode_subset {
    [$p:vis enum $name:ident $tree:tt] => {
        $crate::returncode_with_vis![$p enum $name $tree];
    };
    [enum $name:ident $tree:tt] => {
        $crate::returncode_with_vis![pub(self) enum $name $tree];
    };
}

#[macro_export]
macro_rules! returncode_with_vis {
    [$p:vis enum $name:ident { $($v:ident),* }] => {
        // TODO: Replace with a more efficient manual Debug implementation or
        // some other trait (e.g. ufmt-like).
        #[derive(Debug)]
        $p enum $name { $($v = $crate::returncode_value![$v]),* }
        $($crate::returncode_trait_impl!{$name, $v})*

        // TODO: Evaluate whether we *want* TryFrom.
        impl ::core::convert::TryFrom<isize> for $name {
            type Error = ();  // TODO: Should probably be a custom error type.

            fn try_from(value: isize) -> Result<Self, ()> {
                match value {
                    $($crate::returncode_value![$v] => Ok(Self::$v),)*
                    _ => Err(()),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! returncode_value {
    [SUCCESS] => (0);
    [FAIL] => (-1);
    [EBUSY] => (-2);
    [EALREADY] => (-3);
    [EOFF] => (-4);
    [ERESERVE] => (-5);
    [EINVAL] => (-6);
    [ESIZE] => (-7);
    [ECANCEL] => (-8);
    [ENOMEM] => (-9);
    [ENOSUPPORT] => (-10);
    [ENODEVICE] => (-11);
    [EUNINSTALLED] => (-12);
    [ENOACK] => (-13);
}

#[macro_export]
macro_rules! returncode_trait {
    {SUCCESS}      => ($crate::lw::returncode::Success);
    {FAIL}         => ($crate::lw::returncode::Fail);
    {EBUSY}        => ($crate::lw::returncode::EBusy);
    {EALREADY}     => ($crate::lw::returncode::EAlready);
    {EOFF}         => ($crate::lw::returncode::EOff);
    {ERESERVE}     => ($crate::lw::returncode::EReserve);
    {EINVAL}       => ($crate::lw::returncode::EInval);
    {ESIZE}        => ($crate::lw::returncode::ESize);
    {ECANCEL}      => ($crate::lw::returncode::ECancel);
    {ENOMEM}       => ($crate::lw::returncode::ENoMem);
    {ENOSUPPORT}   => ($crate::lw::returncode::ENoSupport);
    {ENODEVICE}    => ($crate::lw::returncode::ENoDevice);
    {EUNINSTALLED} => ($crate::lw::returncode::EUninstalled);
    {ENOACK}       => ($crate::lw::returncode::ENoAck);
}

// TODO: Figure out how to make these traits work (macro magic).
#[macro_export]
macro_rules! returncode_trait_impl {
    {$n:ident, SUCCESS}      => (impl $crate::lw::returncode::Success      for $n { fn success()      -> Self { Self::SUCCESS      } });
    {$n:ident, FAIL}         => (impl $crate::lw::returncode::Fail         for $n { fn fail()         -> Self { Self::FAIL         } });
    {$n:ident, EBUSY}        => (impl $crate::lw::returncode::EBusy        for $n { fn ebusy()        -> Self { Self::EBUSY        } });
    {$n:ident, EALREADY}     => (impl $crate::lw::returncode::EAlready     for $n { fn ealready()     -> Self { Self::EALREADY     } });
    {$n:ident, EOFF}         => (impl $crate::lw::returncode::EOff         for $n { fn eoff()         -> Self { Self::EOFF         } });
    {$n:ident, ERESERVE}     => (impl $crate::lw::returncode::EReserve     for $n { fn ereserve()     -> Self { Self::ERESERVE     } });
    {$n:ident, EINVAL}       => (impl $crate::lw::returncode::EInval       for $n { fn einval()       -> Self { Self::EINVAL       } });
    {$n:ident, ESIZE}        => (impl $crate::lw::returncode::ESize        for $n { fn esize()        -> Self { Self::ESIZE        } });
    {$n:ident, ECANCEL}      => (impl $crate::lw::returncode::ECancel      for $n { fn encancel()     -> Self { Self::ECANCEL      } });
    {$n:ident, ENOMEM}       => (impl $crate::lw::returncode::ENoMem       for $n { fn enomem()       -> Self { Self::ENOMEM       } });
    {$n:ident, ENOSUPPORT}   => (impl $crate::lw::returncode::ENoSupport   for $n { fn enosupport()   -> Self { Self::ENOSUPPORT   } });
    {$n:ident, ENODEVICE}    => (impl $crate::lw::returncode::ENoDevice    for $n { fn enodevice()    -> Self { Self::ENODEVICE    } });
    {$n:ident, EUNINSTALLED} => (impl $crate::lw::returncode::EUninstalled for $n { fn euninstalled() -> Self { Self::EUNINSTALLED } });
    {$n:ident, ENOACK}       => (impl $crate::lw::returncode::ENoAck       for $n { fn enoack()       -> Self { Self::ENOACK       } });
}

pub trait Success { fn success() -> Self; }
pub trait Fail { fn fail() -> Self; }
pub trait EBusy { fn ebusy() -> Self; }
pub trait EAlready { fn ealready() -> Self; }
pub trait EOff { fn eoff() -> Self; }
pub trait EReserve { fn ereserve() -> Self; }
pub trait EInval { fn einval() -> Self; }
pub trait ESize { fn esize() -> Self; }
pub trait ECancel { fn ecancel() -> Self; }
pub trait ENoMem { fn enomem() -> Self; }
pub trait ENoSupport { fn enosupport() -> Self; }
pub trait ENoDevice { fn enodevice() -> Self; }
pub trait EUninstalled { fn euninstalled() -> Self; }
pub trait ENoAck { fn enoack() -> Self; }
