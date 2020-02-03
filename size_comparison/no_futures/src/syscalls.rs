#[inline(always)]
pub fn command(driver: usize, command_number: usize, arg1: usize, arg2: usize) -> usize {
    let result;
    unsafe {
        asm!("svc 2" : "={r0}"(result)
                     : "{r0}"(driver) "{r1}"(command_number) "{r2}"(arg1) "{r3}"(arg2)
                     : "memory"
                     : "volatile");
    }
    result
}

#[inline(always)]
pub fn subscribe<T>(driver: usize, subscribe_number: usize,
                    callback: unsafe extern "C" fn(usize, usize, usize, Option<&T>),
                    data: Option<&T>) -> usize {
    let result;
    unsafe {
        asm!("svc 1" : "={r0}"(result)
                     : "{r0}"(driver) "{r1}"(subscribe_number) "{r2}"(callback) "{r3}"(data)
                     : "memory"
                     : "volatile");
    }
    result
}

#[inline(always)]
#[cfg(target_arch = "arm")]
pub fn yieldk() {
    // Note: A process stops yielding when there is a callback ready to run,
    // which the kernel executes by modifying the stack frame pushed by the
    // hardware. The kernel copies the PC value from the stack frame to the LR
    // field, and sets the PC value to callback to run. When this frame is
    // unstacked during the interrupt return, the effectively clobbers the LR
    // register.
    //
    // At this point, the callback function is now executing, which may itself
    // clobber any of the other caller-saved registers. Thus we mark this inline
    // assembly as conservatively clobbering all caller-saved registers, forcing
    // yield to save any live registers.
    //
    // Upon direct observation of this function, the LR is the only register
    // that is live across the SVC invocation, however, if the yield call is
    // inlined, it is possible that the LR won't be live at all (commonly seen
    // for the `loop { yieldk(); }` idiom) or that other registers are live,
    // thus it is important to let the compiler do the work here.
    //
    // According to the AAPCS: A subroutine must preserve the contents of the
    // registers r4-r8, r10, r11 and SP (and r9 in PCS variants that designate
    // r9 as v6). Thus we must clobber r0-3, r12, and LR
    unsafe {
        asm!("svc 0"
             :                                                      // Outputs
             :                                                      // Inputs
             : "cc", "memory", "r0", "r1", "r2", "r3", "r12", "lr"  // Clobbers
             : "volatile");
    }
}
