#![no_std]

use core::marker::{PhantomData, PhantomPinned};
use core::mem::size_of_val;
use core::pin::Pin;
use core::ptr::null_mut;
use zerocopy::{FromBytes, IntoBytes};

pub mod allow_ref;
pub mod dynamic_type;
pub mod full_dynamic;
pub mod no_dynamic;

pub type ErrorCode = u32;

// Types to indicate RO allow versus RW allow.
pub enum StaticRo {}
pub enum StaticRw {}
// Should this be sealed?
pub trait StaticType {
    const CLASS: DynamicType;
}
impl StaticType for StaticRo {
    const CLASS: DynamicType = DynamicType::Ro;
}
impl StaticType for StaticRw {
    const CLASS: DynamicType = DynamicType::Rw;
}

#[derive(Clone, Copy)]
pub enum DynamicType {
    Rw = 3,
    Ro = 4,
}

/// Command system call, used mostly to clobber registers to make things more
/// convenient (some of the benchmark code seemed artifically simplified because
/// the compiler was able to reuse registers between Allow calls in an
/// unrealistic way).
pub fn command(driver_num: u32, allow_num: u32, arg0: u32, arg1: u32) -> Result<(), ErrorCode> {
    let [r0, r1];
    unsafe {
        #[cfg(target_arch = "arm")]
        core::arch::asm!(
            "svc 2",
            inlateout("r0") driver_num => r0,
            inlateout("r1") allow_num => r1,
            inlateout("r2") arg0 => _,
            inlateout("r3") arg1 => _,
            options(preserves_flags, nomem, nostack),
        );
        #[cfg(target_arch = "riscv32")]
        core::arch::asm!(
            "ecall",
            inlateout("a0") driver_num => r0,
            inlateout("a1") allow_num => r1,
            inlateout("a2") arg0 => _,
            inlateout("a3") arg1 => _,
            in("a4") 2,
            options(preserves_flags, nomem, nostack),
        );
    }
    match r0 < 128 {
        false => Ok(()),
        true => Err(r1),
    }
}

/// Raw Allow system call with a runtime-specified allow type.
unsafe fn dynamic_allow(
    driver_num: u32,
    allow_num: u32,
    address: *mut u8,
    len: usize,
    allow_type: DynamicType,
) -> (u32, *mut u8, *mut u8, usize) {
    #[cfg(target_arch = "arm")]
    match allow_type {
        DynamicType::Ro => unsafe { static_allow::<StaticRo>(driver_num, allow_num, address, len) },
        DynamicType::Rw => unsafe { static_allow::<StaticRw>(driver_num, allow_num, address, len) },
    }
    #[cfg(target_arch = "riscv32")]
    {
        let (variant, r1, r2, r3);
        unsafe {
            core::arch::asm!(
                "ecall",
                inlateout("a0") driver_num => variant,
                inlateout("a1") allow_num => r1,
                inlateout("a2") address => r2,
                inlateout("a3") len => r3,
                in("a4") allow_type as u32,
                options(preserves_flags, nostack),
            );
        }
        (variant, r1, r2, r3)
    }
}

/// Raw Allow system call with a `const` allow type.
unsafe fn static_allow<T: StaticType>(
    driver_num: u32,
    allow_num: u32,
    address: *mut u8,
    len: usize,
) -> (u32, *mut u8, *mut u8, usize) {
    #[cfg(target_arch = "arm")]
    {
        let (variant, r1, r2, r3);
        unsafe {
            core::arch::asm!(
                "svc {CLASS_NUMBER}",
                inlateout("r0") driver_num => variant,
                inlateout("r1") allow_num => r1,
                inlateout("r2") address => r2,
                inlateout("r3") len => r3,
                options(preserves_flags, nostack),
                CLASS_NUMBER = const T::CLASS as u8,
            );
        }
        (variant, r1, r2, r3)
    }
    #[cfg(target_arch = "riscv32")]
    unsafe {
        dynamic_allow(driver_num, allow_num, address, len, T::CLASS)
    }
}

/// Required for the examples to compile.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
