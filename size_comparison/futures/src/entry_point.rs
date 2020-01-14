/// Tock OS runtime library. Sets up the stack and data regions before calling
/// into main().

// start and rust_start are the first two procedures executed when a Tock
// application starts. start is invoked directly by the Tock kernel; it performs
// stack setup then calls rust_start. rust_start performs data relocation before
// calling the rustc-generated main. rust_start and start are tightly coupled.
//
// This entry point is designed to work with the libtock-rs linker script
// (layout.ld).
//
// When the kernel gives control to start, it passes four arguments as follows:
//
//     +--------------+ <- (3) memory_len
//     | Grant        |
//     +--------------+
//     | Unused       |
//  S  +--------------+ <- (4) app_break
//  R  | Heap         |        (hardcoded to mem_start + 3072 in
//  A  +--------------|        Process::create which could be less than
//  M  | .bss         |        mem_start + stack + .data + .bss)
//     +--------------|
//     | .data        |
//     +--------------+
//     | Stack        |
//     +--------------+ <- (2) memory_start
//
//  F  +--------------+
//  L  | .text        |
//  A  +--------------+ <- (1) flash_app_start
//  S  | Protected    |
//  H  | Region       |
//     +--------------+
//
// We want to organize the memory as follows:
//
//     +~~~~~~~~~~~~~~+
//     | Heap         |
//     +--------------| <- heap_start
//     | .bss         |
//     +--------------|
//     | .data        |
//     +--------------+ <- stack_start (stacktop)
//     | Stack        |
//     | (grows down) |
//     +--------------+ <- memory_start

#[no_mangle]
#[naked]
#[link_section = ".start"]
#[cfg(target_arch = "arm")]
unsafe extern "C" fn start(
    _flash_app_start: usize,
    _memory_start: usize,
    _memory_len: usize,
    _app_break: usize,
) -> ! {
    asm!("
        // An offset between the location the program is linked at and its
        // actual location in flash would cause references to .rodata to point
        // to the wrong data. To mitigate this, this section checks that .start
        // is loaded at the correct location. If the application was linked and
        // loaded correctly, pc will match the intended location of .start. If
        // they do not match, the low level debug driver will be used to signal
        // an error, and we'll jump to the yield loop.
        sub r0, pc, #4    // r0 = pc
        ldr r1, =.start   // r1 = address of .start
        cmp r0, r1
        beq .Lstack_init  // Jump to stack initialization if pc was correct
        movw r0, #8       // LowLevelDebug driver number
        movw r1, #1       // LowLevelDebug 'print status code' command
        movw r2, #2       // LowLevelDebug relocation failed status code
        svc 2             // command() syscall
        b .Lyield_loop

        .Lstack_init:
        // Move the app break to the top of .bss, guaranteeing we have enough
        // room for the stack, .data, and .bss.
        movw r0, #0          // memop() brk operation
        ldr r1, =heap_start  // r1 = heap_start
        svc 4                // memop() syscall
        // Set the stack pointer.
        ldr sp, =stack_top   // sp = stack_top

        // Call rust_start
        bl rust_start

        // Yield loop. This is used if rust_start returns or if the location
        // check at the start of this assembly fails. It calls the yield syscall
        // in an infinite loop.
        .Lyield_loop:
        svc 0
        b .Lyield_loop"
        :                                                // No output operands
        :                                                // Input operands
        : "r0", "r1", "r2", "r3", "r12", "cc", "memory"  // Clobbers
        : "volatile"                                     // Options
    );

    // start() should not return, but asm!() returns (). unreachable_unchecked()
    // seems to be the safest way to get the ! return type we need.
    core::hint::unreachable_unchecked()
}

/// Rust setup, called by start. Uses the extern "C" calling convention so that
/// the assembly in start knows how to call it (the Rust ABI is not defined).
/// Sets up the data segment (including relocations) and the heap (if enabled),
/// then calls into the rustc-generated main(). This cannot use mutable global
/// variables or global references to globals until it is done setting up the
/// data segment.
#[no_mangle]
unsafe extern "C" fn rust_start() -> () {
    use core::ptr::copy_nonoverlapping;

    extern "Rust" {
        static rt_header: RtHeader;
        static data_flash_start: EmptySymbol;

        static data_ram_start: EmptySymbol;
    }

    // Initialize .data and .bss
    copy_nonoverlapping(data_flash_start.as_ptr_u8(),
        data_ram_start.as_mut_u8(), rt_header.data_size);
    core::ptr::write_bytes(rt_header.bss_start, 0, rt_header.bss_size);

    extern "C" {
        // This function is created internally by `rustc`. It calls the
        // application-defined main().
        fn main(argc: isize, argv: *const *const u8) -> isize;
    }
    main(0, core::ptr::null());
}

/// The header encoded at the beginning of .text by the linker script. It is
/// accessed by rust_start() using its flash_app_start parameter.
#[repr(C)]
struct RtHeader {
    data_size: usize,
    bss_start: *mut u8,
    bss_size: usize,
}

/// The linker script defines several symbols whose locations are meaningful,
/// but which don't point at any data (have size zero). This is a Rust type that
/// corresponds to those symbols. It exposes utility functions that return the
/// symbol's location in the types that rust_start() needs.
#[repr(C)]
struct EmptySymbol {}

impl EmptySymbol {
    fn as_ptr_u8(&self) -> *const u8 {
        self as *const EmptySymbol as *const u8
    }

    fn as_mut_u8(&self) -> *mut u8 {
        self as *const EmptySymbol as *mut u8
    }
}
