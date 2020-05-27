# Asynchronous Components using Zero Sized Type Pointers

Original author: Johnathan Van Why

This document introduces a design (the "ZST-pointer" design) for asynchronous
APIs in `libtock-rs`. It then gives an analysis of the design's tradeoffs, and
the results of an experiment evaluating how this API design can live alongside
a futures-based asynchronous API.

## Background

`libtock-rs` should expose asynchronous interfaces to the Tock kernel’s
asynchronous APIs. There are a variety of mechanisms a library can use to expose
an asynchronous API, with varying tradeoffs. The Rust community has settled on
using Rust’s [Future](https://doc.rust-lang.org/core/future/trait.Future.html)
trait to construct asynchronous APIs. Unfortunately, the analysis at [Futures
versus no-Futures Size Comparison](../size_comparison) shows that the Future
trait has size costs that are too high for many use cases of `libtock-rs`. As a
result, we need an alternative design for `libtock-rs`’s asynchronous APIs.

This document presents the "ZST-pointer" design, a candidate for the design of
those asynchronous system call APIs.

## Objectives of this design

We have set the following objectives for the design of the system call APIs:

1. **Asynchronous:** They must support asynchronous operation, as Tock OS has an
   asynchronous design and many userspace binaries will need to perform
   concurrent operations.
1. **Lightweight:** To maximize the amount of functionality that can be
   incorporated into a single Tock board, Tock applications must occupy minimal
   flash and RAM space. Ideally, the abstractions required to support
   asynchronous operation would have no impact on binary size, RAM usage, or
   execution time.
1. **Incremental migration from futures:** If we develop an
   expensive-but-convenient futures API and an efficient-but-inconvenient API
   independently, then it becomes difficult for application developers to choose
   what API to build their applications against. Choosing the
   efficient-but-inconvenient API for non-constrained applications wastes
   engineering effort. Choosing the expensive-but-convenient API for a new
   application runs the risk of requiring an application rewrite down the road
   when the application grows large. To make this decision more reasonable, and
   to prevent large application rewrite projects, we want to make it possible to
   incrementally migrate a large application from the futures-based API to the
   new API.

There were a few other considerations as well:

*  I tried to design the asynchronous APIs using only stable Rust features.
*  Testability: we want to thoroughly unit-test (and occasionally
   integration-test) all system call interfaces. This mostly means making the
   API design dependency injection friendly.
*  Understandability/maintainability.

## ZST Pointer Design

The design is based on the Tock kernel's asynchronous constructs, and keeps two
of its major concepts:

1. **A static graph of interacting state machines:** The Tock kernel has a
   statically-specified graph of objects (as in OOP) that interact with each
   other. It is uncommon to use dynamic handles (which is the norm when using
   futures), which minimizes overhead. It is normal for objects' interfaces to
   represent state machines, and for their clients to rely on that state
   machine's behavior to minimize the amount of duplicate state kept in RAM.
1. **Split-phase calls:** If object `A` wants to invoke an asynchronous
   operation implemented by object `B`, `A` calls a method on `B` to start the
   operation, which returns without blocking, then `B` calls `A` back when the
   operation is finished.

For example, in the Tock kernel, a connection between an object `Bar` and an
interface it depends on `Foo` would look like:

```
// The API Bar depends on.
trait Foo {
    fn foo(&self);
}

// Implemented by Bar; contains a callback called when the Foo::foo operation is
// complete.
trait FooClient {
    fn foo_done(&self);
}

// Implemented by objects using Bar, contains a callback called when Bar's
// operation completes.
trait BarClient {
    fn bar_done(&self);
}

// The `Bar` object. Note that the Foo's type is injected statically, but a
// reference to the Foo is injected dynamically. Bar's client is injected fully
// dynamically.
struct Bar<F: Foo> {
    foo: &F,  // 1 word
    client: &dyn BarClient,  // 2 words
}

impl<F: Foo> FooClient for Bar<F> { ... }
```

The Tock kernel's implementation has unnecessary overhead. Objects in the Tock
kernel contain references to their dependencies (objects that implement APIs
that they need), even though those dependencies have locations known at compile
time. For example, `LowLevelDebug` carries a `&uart::Transmit`, which costs 1
word in RAM. Worse, the callback portion of the split-phase calls is generally
implemented with `&dyn` references, which has the following costs:

1. 2 words of space in RAM (data pointer + vtable pointer)
1. A vtable in flash with 3 words Tock doesn't use (size, alignment, and
   destructor).
1. Prevents those calls from being inlined. LLVM has devirtualization to
   alleviate this but it is not very effective in practice (in part because it
   operates on bitcode types rather than Rust types). See also
   https://github.com/rust-lang/rust/issues/45774

In the proposed design, `Bar` does not store a reference to a `Foo`, it stores a
type that can forward calls to the `Foo`. That type is injected as a dependency
of `Bar` via static polymorphism. Similarly, the `BarClient` is injected via
dependency injection:

```
// FooPtr is implemented by a type that knows how to find an implementation of
// `Foo`.
trait FooPtr: Copy {
    // If FooPtr is zero-sized, then the `self` argument has no cost.
    fn foo(self);
}

// FooClient is unchanged from the previous example.
trait FooClient {
    fn foo_done(&self);
}

// BarClientPtr is implemented by a type that knows how to find an
// implementation of `BarClient`. Like FooPtr, BarClientPtr may be implemented
// as a zero-sized type.
trait BarClientPtr: Copy {
    fn bar_done(self);
}

trait BarDependencies : FooPtr + BarClientPtr {}

struct Bar<D: BarDependencies> {
    deps: BarDependencies,  // May be zero-sized.
}

impl<D: BarDependencies> FooClient for Bar<D> { ... }
```

`FooPtr` and `BarClientPtr` would be implemented by the code that instantiates
the `Foo` and the `BarClient`, respectively. If the `Foo` and `BarClient` are
`static` items, then `FooPtr` and `BarClientPtr` can be zero-sized types and
carry no run-time overhead. The downside of this approach is the increased
boilerplate relative to the kernel's approach (for the `FooPtr`, `BarClientPtr`,
and `BarDependencies` implementations).

Because the API replaces Rust's native reference types with new types that have
similar functionality but are zero-sized types (ZSTs), I'm calling this the "ZST
Pointer" approach to asynchronous APIs.

In practice, there are a lot of variations on this approach that are possible,
such as using generic traits to represent the *-Ptr types (discussed below). My
implementation of this approach is in the [lw/](lw/) directory. My
implementation of a futures-based API on top of the lightweight API is in the
[futures/](futures/) directory. Neither module has been cleaned up for
readability; they are contained as a research archive rather than as an example.

## `Static` and `Sync`

Buffers and callbacks given to the kernel must be of `static` lifetime. As a
result, we need a way to store the userspace API objects in `static` variables.
`static mut` is unsafe, and there is a
[proposal](https://github.com/rust-lang/rust/issues/53639) to deprecate it from
the language. Like the Tock kernel's coding style, the ZST pointer coding style
makes heavy use of shared references, which are not `Sync`. In Rust, `static`
items must be `Sync` (for thread safety), so we needed an additional abstraction
to prevent the proliferation of `unsafe`.

### Approach 1: `TockStatic<T>`

`Sync` is only relevant in a multithreaded context, and `libtock-rs`'s runtime
is (currently) single-threaded. With `libtock-rs`'s runtime, it is sound to
implement `Sync` for any type. To implement this, we can introduce a wrapper
type `TockStatic<T>` that allows us to make any type `Sync`:

```
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
```

It is important that the above type only be exposed to crates that link in
`libtock-core`'s runtime, as it is wildly unsafe in multithreaded environments!

`TockStatic<T>` carries the downside that its API *cannot* be implemented
soundly in multithreaded environments, which would become a problem if we decide
to add a multithreaded runtime to `libtock-rs`.

### Approach 2: `SyncCell<T>`

After implementing `TockStatic<T>` and using it for a while, I started seeing
the following pattern appear repeatedly: `TockStatic<Cell<T>>`. I realized that
most of `core::cell::Cell`'s API could be implemented in a threadsafe manner by
disabling thread switching, with no impact on the size of the object. Disabling
thread switching for all operations on the cell would prevent any thread
accessing that cell from being preempted by another application thread,
preventing concurrent accesses to the same data.

Based on this observation, I introduced `SyncCell<T>`, which is essentially
`Cell<T>` but implements `Sync`. `SyncCell` is implemented in
[sync_cell.rs](lw/sync_cell.rs). `SyncCell` omits some casting-style methods
that are not compatible with a mutex-based implementation.

### Approach 3: Mutexes

Alternatively, `libtock-rs`'s runtime(s) could expose mutex types that userspace
code uses to safely store types in `static` items. Because a mutex's API is less
pleasant to use than the above options (and that verbose code would be pervasive
in `libtock-rs` and application code), I opted not to expose a mutex-like API in
my prototypes.

## Incremental Migration Experiment

The above implementation is -- by design -- asynchronous and lighter-weight than
the kernel's code structure. However, it is not immediately obvious that a large
application can be migrated from a futures-based API to the ZST pointer API. To
test this, I implemented a futures-based API on top of the ZST pointer API,
ported [OpenSK](https://www.github.com/google/OpenSK) to it, then incrementally
migrated OpenSK to the ZST pointer API.

### Learnings

The futures implementation needs to use virtualized versions of the underlying
drivers. If, for example, the futures API does not use a virtualized button
driver, then all uses of the buttons API will need to be migrated from the
futures-based API to the ZST pointer API simultaneously. When the buttons
futures are used in futures combinators, that will cause a cascade where other
API usages would need to be migrated at the same time. Building the futures API
atop a virtualized ZST pointer API prevents this cascade.

Once I migrated the futures to use virtualized versions of the underlying
drivers, completing the incremental migration was straightforward.

## Additional Ideas

### Generic Traits

Instead of making each object that exposes asynchronous methods add 4 traits
(the object's API, a zero-sized pointer to the object, the client API, and a
pointer to the client), we can instead use a fixed set of generic traits.

The first trait is implemented by objects that support an asynchronous call:

```
pub trait AsyncFn<Args, SyncOutput> {
    fn call(&self, args: Args) -> SyncOutput;
}
```

The second is implemented by zero-sized references to the above objects:

```
pub trait AsyncFnPtr<Args, SyncOutput> : Copy {
    fn call(self, args: Args) -> SyncOutput;
}
```

The third is implemented by clients of objects that support an asynchronous
call, and contains the second phase of the split-phase call pattern:

```
pub trait AsyncClient<AsyncOutput> {
    fn callback(&self, output: AsyncOutput);
}
```

The last is implemented by lightweight pointers to `AsyncClient`s:

```
pub trait AsyncClientPtr<AsyncOutput> : Copy {
    fn callback(self, output: AsyncOutput);
}
```

Introducing these traits would add cognitive overhead to `libtock-rs`:
understanding `libtock-rs` code (or apps written on top of `libtock-rs`) would
require understanding these traits and how they relate to each other. However,
they are useful in several ways:

1.  They would reduce the amount of boilerplate in `libtock-rs` significantly.
1.  The client traits are needed in order to write a `&dyn`-like abstraction for
    virtualization layers. I implemented this idea as `DynCall` in
    [lw/async_util.rs](lw/async_util.rs).
1.  They allow the use of generic impls to generate the zero-sized pointer
    types.

We should probably have some form of these traits in `libtock-rs`.

### Yieldable/Callback Context Types

Allowing objects to call into each other arbitrarily has some issues. The Tock
kernel developers have already experienced unintentional reentrancy and
unbounded recursion. If object `A` tries to start an async operation on object
`B`, but that operation fails to start, what happens if `B` calls `A`'s callback
immediately? In practice, `A` isn't usually expecting that call, and it can lead
to bugs (including infinite recursion). On the other hand, if `B` calls back
into `A` is `A` allowed to immediately start a new operation in `B`? That seems
useful.

In practice, the Tock kernel solves this by asking objects to never call their
second-phase callbacks in a first-phase context. For cases where `B` needs to
generate a callback but is not expecting a later interrupt/callback, there is a
deferred call mechanism for `B` to ask for a callback.

We can introduce a zero-sized `CallbackMarker` type that is passed to deferred
and kernel callbacks. Then second-phase callbacks could require a
`CallbackMarker` to prevent them from being accidentally called from a
first-phase context. This would involve the following change to the above
traits:

```
pub trait AsyncClient<AsyncOutput> {
    fn callback(&self, callback_marker: CallbackMarker, output: AsyncOutput);
}

pub trait AsyncClientPtr<AsyncOutput> : Copy {
    fn callback(self, callback_marker: CallbackMarker, output: AsyncOutput);
}
```

`CallbackMarker` could be defined by the crate/module that supplies the system
call and deferred call implementation, with no public constructor. Then it would
not be possible for application-level code to incorrectly create a
`CallbackMarker`.

A similar problem exists for calling `yield()` (which should only be called from
synchronous contexts). However, there isn't as elegant a solution, because user
code needs to be able to call into `yield()` from main, which cannot accept
arguments.
