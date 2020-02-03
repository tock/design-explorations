#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {
        crate::syscalls::yieldk();
    }
}

#[lang = "start"]
fn start<T>(main: fn() -> (), _: isize, _: *const *const u8) {
    main();
}

#[lang = "termination"]
pub trait Termination {}
impl Termination for () {}
