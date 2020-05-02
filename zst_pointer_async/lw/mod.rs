pub mod async_util;
pub mod button;
pub mod console;
pub mod deferred;
pub mod dyncall;
pub mod led;
pub mod returncode;
pub mod rng;
pub mod sync_cell;
pub mod time;
pub mod virt_time;

// TODO: Figure out a way to initialize drivers that want or need runtime
// initialization (e.g. calling subscribe() to set up static callbacks). We may
// be able to use some sort of "init token" system (implemented kinda like
// capabilities) to show that a driver is initialized -- but note that the
// system cannot be trusted for safety unless we have a way to show the
// particular instance was initialized. Even then, for subscriptions different
// instances can overwrite each other, although that shouldn't result in safety
// issues. That said, subscriptions may show up in the syscall traits, so
// subscription initialization and clashes may not be a problem in the first
// place.

// TODO: Idea for an init system. Each init() method returns one token type.
// Each method that requires initialization requires a second token type.
// Constructing the second token type requires providing copies of the first
// token type for each dependency of the component being initialized. I.e. if A
// depends on B and C, then invoking A::do_thing() requires a token whose
// construction requires the result of A::init(), B::init(), and C::init().
// Doesn't solve the "exact same instance" problem, but I don't think that can
// be solved. Still unclear whether it's better to have an init system that
// doesn't guarantee the *correct* instances are initialized or to have no
// forced init system.
