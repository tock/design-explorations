# Typed Syscall Data

## Motivation

The primary goal of this document is to promote a more type-safe syscall API for
future major Tock versions (3.0 and beyond). It does so by proposing a syscall
ABI and examining its pros and cons. A secondary goal is to spread knowledge
about CHERI among core Tock developers.

To date, Tock's [syscall
ABI](https://github.com/tock/tock/blob/master/doc/reference/trd104-syscalls.md)
has only been defined for 32-bit non-CHERI platforms. This has allowed Tock's
syscall ABI to use `u32`, `usize`, and pointers relatively interchangeably.
However, there is now interest in porting Tock to CHERI platforms, both 32-bit
and 64-bit. Suddenly, `u32`, `usize`, and `*mut ()` can be different sizes (in
64-bit purecap CHERI, they are three distinct sizes!), so we can no longer treat
them as equivalent.

[TRD
104](https://github.com/tock/tock/blob/master/doc/reference/trd104-syscalls.md)
supports returning three data types from system calls: `ErrorCode`, `u32`, and
`u64`. Its list of return variants contains every combination of:

1. 0 or 1 `ErrorCode`s
2. `u32`
3. `u32`

That can fit into four registers. That results in 10 system call return variants
(4 failures and 6 successes).

To support 64-bit architectures and CHERI,
https://github.com/tock/tock/pull/4174 extends the number of supported data
types to five, adding "address" and "pointer" to the list. Adding those types to
our list enables us to expand the number of return variants considerably:

* **Failures:** Failure, Failure with u32, Failure with address, Failure with
  pointer, Failure with 2 u32, Failure with u32 and address, Failure with u32
  and pointer, Failure with 2 address, Failure with address and pointer, Failure
  with 2 pointer, Failure with u64.
* **Successes:** Success, Success with u32, Success with address, Success with
  pointer, Success with 2 u32, Success with u32 and address, Success with u32
  and pointer, Success with 2 address, Success with address and pointer, Success
  with 2 pointer, Success with u64, Success with 3 u32s, Success with 2 u32s and
  address, Success with 2 u32 and pointer, Success with u32 and 2 address,
  Success with u32 and address and pointer, Success with 3 address, Success with
  2 address and pointer, success with address and 2 pointer, Success with 3
  pointer, Success with u32 and u64, Success with address and u64, Success with
  pointer and u64.

I've probably made a mistake, but if I got it correct, that is 11 failure types
and 22 success types! Supporting all of these possibilities is starting to
become impractical; a `match` over the full list of 33 possibilities is going to
be error-prone to implement by hand and likely generate a lot of code. Indeed
https://github.com/tock/tock/pull/4174 only adds the return variants that it
immediately needs: Success with address and Success with pointer.

At a higher level, Tock frequently relies on syscall drivers and userspace
libraries to cast types for transfer across the syscall interface. For example,
the temperature capsule [uses an unsigned upcall argument to send an `i32` to
userspace](https://github.com/tock/tock/blob/772ed33c594cb3fcd7590444a6b45aaca1172b68/capsules/extra/src/temperature.rs#L131),
the buttons capsule [uses SuccessWithU32 to return a boolean
value](https://github.com/tock/tock/blob/772ed33c594cb3fcd7590444a6b45aaca1172b68/doc/syscalls/00003_buttons.md#command-number-3),
and the console driver [passes an error code in an integer argument of
upcalls](https://github.com/tock/tock/blob/772ed33c594cb3fcd7590444a6b45aaca1172b68/doc/syscalls/00001_console.md#subscribe-number-2).
Tock's syscall APIs do not help users perform these casts correctly.

A more extensible syscall ABI would allow us to support a larger variety of
argument and return types without generating excessive overhead.

## What types do we care about?

To start designing the syscall ABI, we can look at the [list of all Rust
types](https://doc.rust-lang.org/reference/types.html) and see several types
that probably make sense in a syscall interface:

* `bool`
* Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`
* Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
* Floating point: `f32`, `f64`
* Codepoint: `char`
* Upcall function pointer
* Data pointers: `*const T`, `*mut T`

Note about pointers: if `T` is not `Sized`, then `*const T` and `*mut T` are
larger than normal pointers and are known as wide pointers. Wide pointers do not
have a stable ABI, so we won't use them in the syscall ABI. For the rest of the
document, assume that `*const T` and `*mut T` only point to `Sized` types.

There are a couple other types that we probably want to consider as well:

* Tock's `ErrorCode`, as it is extremely common.
* A non-pointer CHERI capability: not all CHERI capabilities are pointers. For
  example, an OS could use CHERI capabilities as file handles to prevent
  userspace programs from forging file handles. Tock does not currently have a
  use for non-pointer capabilities, but this document includes them for
  educational and future-proofing purposes.
* A Register type that represents *any* possible register value. This is a
  future-compatibility safeguard: if we ever need a type that is not in our
  fixed list, we can call it a Register and make things work (albeit with less
  type safety). It is also used by Yield to pass dynamically-typed data to
  userspace.

### Reducing the number of types

Of course, there is a cost to complexity and in particular supporting a long
list of types. It probably doesn't make sense to provide special support for
*all* of the above types. In particular:

* `i8` and `i16` are not particularly common and can be passed as `i32` without
  risking data corruption.
* `u8` and `u16` can similarly be passed as `u32`.
* `i128` and `u128` are extremely rare in Rust, completely absent in standard C,
  and probably exceptionally rare on the small systems Tock targets.
* `char` is rare in Tock, as Tock mostly treats text as byte buffers. When
  needed, it can be passed as `u32`. It's also not a standard part of C (where
  `char` has a different meaning).
* `*const T` and `*mut T` are redundant: they're distinct types, but only exist
  to communicate mutability information. We already handle this in Tock by using
  different system calls for buffers of different mutability. Therefore we only
  need one pointer type.

### The reduced list

This leaves us with:

* `bool`
* Numeric: `i32`, `i64`, `u32`, `u64`, `f32`, `f64`
* Sizes and offsets: `usize`, `isize`
* Upcall function pointer
* Data pointer (`*const T` or `*mut T`)
* `ErrorCode`
* Non-pointer CHERI capability

CHERI note: on CHERI systems, both function pointers and data pointers are
passed across the syscall ABI as CHERI capabilities.

## Type descriptors

When a process wants to send data to the kernel, the process needs a way to tell
the kernel the sent data's type (and vice versa when the kernel sends data to
the process). To do this, we need a way to serialize information about a a list
of types. To start, lets assign numbers to each type (DNE means this type Does
Not Exist yet):

| ID       | Description                  | Kernel Type     | `libtock-c` Type | `libtock-rs` Type |
| -------- | ---------------------------- | --------------- | ---------------- | ----------------- |
| `0b0000` | None (invalid)               | -               | -                | -                 |
| `0b0001` | Error code                   | `ErrorCode`     | DNE              | `ErrorCode`       |
| `0b0010` | `u32`                        | `u32`           | `uint32_t`       | `u32`             |
| `0b0011` | `i32`                        | `i32`           | `int32_t`        | `i32`             |
| `0b0100` | `usize`                      | `usize`         | `size_t`         | `usize`           |
| `0b0101` | `isize`                      | `isize`         | `ptrdiff_t`      | `isize`           |
| `0b0110` | `u64`                        | `u64`           | `uint64_t`       | `u64`             |
| `0b0111` | `i64`                        | `i64`           | `int64_t`        | `i64`             |
| `0b1000` | `f32`                        | `f32`           | `float`          | `f32`             |
| `0b1001` | `f64`                        | `f64`           | `double`         | `f64`             |
| `0b1010` | `bool`                       | `bool`          | `bool`           | `bool`            |
| `0b1011` | Upcall pointer               | `CapabilityPtr` | `UpcallFn`       | `UpcallFn`        |
| `0b1100` | Data pointer                 | `CapabilityPtr` | `T*`             | `*mut T`          |
| `0b1101` | Non-pointer CHERI capability | DNE             | DNE              | DNE               |
| `0b1110` | *Reserved for future use*    | -               | -                | -                 |
| `0b1111` | Arbitrary register value     | DNE             | DNE              | `Register`        |

We can describe a list of N types as a 4N bit integer by embedding the Nth type
ID in the Nth nibble of the integer. So:

* The empty list has ID `0`
* A list of one type has an ID equal to that type.
* A list of two types has the first type in bits 0-3 and the second in bits 4-7.
* A list of three types has the first type in bits 0-3, the second in bits 4-7,
  and the third in bits 8-11.
* and so on

For example, the type `(bool, u32, *mut T)` would be described by
`0b110000101010`, expanded here:

| Bits                    | Value    | Type         |
| ----------------------- | -------- | ------------ |
| 0-3 (least significant) | `0b1010` | `bool`       |
| 4-7                     | `0b0010` | `u32`        |
| 8-11                    | `0b1100` | Data pointer |

Note that if this type descriptor were stored in a larger type (such as a
`u32`), you can determine that it is a list of three types because `0b0000` is
not a valid type ID.

## Putting multiple differently-typed values into registers

This table indicates how many registers are needed for each type:

| Type              | 32 bit non-CHERI | 64 bit non-CHERI | 32 bit CHERI | 64 bit CHERI |
| ----------------- | ---------------- | ---------------- | ------------ | ------------ |
| `u64`             | 2                | 1                | 2            | 1            |
| `i64`             | 2                | 1                | 2            | 1            |
| `f64`             | 2                | 1                | 2            | 1            |
| CHERI capability  | N/A              | N/A              | 1            | 1            |
| *Everything else* | 1                | 1                | 1            | 1            |

Shorthand: we'll use `regcount<T>` to denote the number of registers needed for
type `T` on a particular platform.

If we have a list of typed values `(v1: T1, v2: T2, v3: T3, ...)` and an ordered
list of registers, we can store `v1` in the first `regcount<T1>` registers, `v2`
in the next `regcount<T2>` registers, `v3` in the next `regcount<T3>` registers,
and so on. Values that span multiple registers store their least significant
bits in the first register and their most significant bits in the second
register. To make this concrete, if we have values `(v1: bool, v2: u64, v3: *mut
())` we would pack them as follows on a 32 bit non-CHERI system:

1. `v1`
2. Least-significant 32 bits of `v2`
3. Most-significant 32 bits of `v2`
4. `v3`

## ArbitraryData

`ArbitraryData` is a data structure designed to fit in registers and carry data
of a variety of types. Like [TRD 104's return
variants](https://github.com/tock/tock/blob/a1966b8ddaa1ce819b80f2f7bea466eb76e5b46c/doc/reference/trd104-syscalls.md#32-return-values),
it embeds information about what data type(s) it carries into one of the
registers. However, it supports a much larger variety of data types than TRD
104's return variants, and is also suitable for passing data to the kernel from
userspace.

The value of the first register in an `ArbitraryData` is the type descriptor for
the list of data types it contains. The remaining pieces of data are stored in
the rest of the registers in order (as described in the previous heading).

For example, on a non-CHERI 32-bit system, `ArbitraryData` would store the value
`(true, 0x0123456789ABCDEFu64, 3i32)` as:

| Register | Value            | Meaning                                |
| -------- | ---------------- | -------------------------------------- |
| 0        | `0b001101101010` | Type descriptor for `(bool, u64, i32)` |
| 1        | `1`              | `true`                                 |
| 2        | `0x89ABCDEF`     | Lower 32 bits of `0x0123456789ABCDEF`  |
| 3        | `0x01234567`     | Upper 32 bits of `0x0123456789ABCDEF`  |
| 4        | `3`              | `3`                                    |

We also use `ArbitraryData(N)` (where N >= 1) to denote an `ArbitraryData` that
must fit into N registers. If the `ArbitraryData` only uses the first K
registers to store data, then the last N-K registers are left unspecified and
should not be read.

## Register names

We'll use the names `a1`, `a2`, `a3`, etc. to refer to registers used to pass
syscall arguments to the kernel, and the names `r1`, `r2`, `r3`, etc. to refer
to registers used to return syscall results to userspace. These names map to
ARM and RISC-V registers via the following tables:

|        |  a1 |  a2 |  a3 |  a4 |  a5 |  a6 |  a7 |  a8 |  a9 | a10 | a11 | a12 | a13 | a14 | a15 | a16 | a17 | a18 | a19 | a20 | a21 | a22 | a23 | a24 | a25 |
| ------ | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ARM    |  a1 |  a2 |  a3 |  a4 |  v1 |  v2 |  v3 |  v4 |  v5 |  v7 |  v8 |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| RISC-V | x11 | x12 | x13 | x14 | x15 | x16 | x17 |  x5 |  x6 |  x7 | x28 | x29 | x30 | x31 |  x9 | x18 | x19 | x20 | x21 | x22 | x23 | x24 | x25 | x26 | x27 |

|        |  r1 |  r2 |  r3 |  r4 |  r5 |  r6 |  r7 |  r8 |  r9 | r10 | r11 | r12 | r13 | r14 | r15 | r16 | r17 | r18 | r19 | r20 | r21 | r22 | r23 | r24 | r25 | r26 |
| ------ | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ARM    |  a1 |  a2 |  a3 |  a4 |  v1 |  v2 |  v3 |  v4 |  v5 |  v7 |  v8 |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| RISC-V | x10 | x11 | x12 | x13 | x14 | x15 | x16 | x17 |  x5 |  x6 |  x7 | x28 | x29 | x30 | x31 |  x9 | x18 | x19 | x20 | x21 | x22 | x23 | x24 | x25 | x26 | x27 |

An ARM, the system call being invoked is passed via the `svc` instruction; on
RISC-V it is passed via `x10` (this is why `x10` is a return register but not
an argument register).

Convention: In this document I will often list a range of registers, such as
r1-r15. Whenever such a range is specified, it means the list of registers that
are in that range and exist on the platform a particular Tock system is running
on. For example, r10-r13 refers to (x6, x7, x28, x29) on RISC-V but only (v7,
v8) on ARM because r12 and r13 do not exist on ARM.

## Proposed Syscall ABI

This section proposes a system call ABI for Tock. This system call ABI is based
on [TRD
104](https://github.com/tock/tock/blob/master/doc/reference/trd104-syscalls.md)
and omits many details that are either the same as TRD 104 or which are
irrelevant to exploring the concept of typed system calls.

### Return Values

All system calls return an `ArbitraryData(N)`, where `N` is the maximum of:

1. The number of registers required to return the Success variant.
1. The number of registers required to return the Failure variant.
1. The number of registers required to return a `(ErrorCode)`.

Note: All system calls must specify their success and failure variant, so that
userspace libraries can determine how many registers may be clobbered by the
syscall. Also, the Success and Failure variants must be different so that
userspace can determine whether the system call has succeeded. In practice,
Failure variants will contain `ErrorCode` and Success variants generally will
not contain `ErrorCode`.

If userspace tries to invoke a system call that the kernel does not recognize or
does not support, or a system call on a system call driver that does not exist,
the kernel will return type `(ErrorCode)`.

### Upcall arguments

Semantically, upcalls have two arguments: the userdata pointer (which was passed
to Subscribe), and an `ArbitraryData(4)` of data from the syscall driver. This
allows syscall drivers to pass up to three values to each invoked upcall. Upcall
functions must have an ABI compatible with the following signature (specified in
both Rust and C):

```C
struct UpcallArbitraryData {
    Register registers[4];
}

typedef void(void*, UpcallArbitraryData) UpcallFn;
```

```Rust
#[repr(C)]
struct UpcallArbitraryData {
    registers: [Register; 4],
}

type UpcallFn = unsafe extern "C" fn(*mut (), UpcallArbitraryData);
```

### Yield

Arguments:

| Yield type | a1  | a2            | a3               |
| ---------- | --- | ------------- | ---------------- |
| no-wait    | `0` | Unused        | Unused           |
| wait       | `1` | Unused        | Unused           |
| wait-for   | `2` | Driver number | Subscribe number |

If there is no pending upcall, yield-no-wait will return `()`.

If there is an upcall to invoke, yield-wait and yield-no-wait return `(upcall fn
pointer, userdata: *mut (), Register, Register, Register, Register)`. The
userspace Yield function should then pack the register values into an
`UpcallArbitraryData` and invoke the function pointer with the userdata pointer
and `UpcallArbitraryData` as parameters.

Note: Unlike TRD 104, in this syscall ABI Yield returns the upcall's arguments
to the userspace library and expects the userspace library to invoke the
callback. This allows for more than four registers' worth of arguments to be
passed to the upcall without requiring the kernel to push arguments onto the
userspace stack.

If there is an upcall, yield-wait-for returns `(Register, Register, Register,
Register)`. The four `Register` values are the upcall's `ArbitraryData(4)`
parameters.

### Subscribe

Arguments:

| Register | Argument         | Type               |
| -------- | ---------------- | ------------------ |
| a1       | Driver number    | `u32`              |
| a2       | Subscribe number | `u32`              |
| a3       | Upcall pointer   | Upcall fn pointer  |
| a4       | Application data | `*mut ()`/`void *` |

Return variants:

| Outcome | Type                                                 |
| ------- | ---------------------------------------------------- |
| Failure | (`ErrorCode`, upcall fn pointer, `*mut ()`/`void *`) |
| Success | (upcall fn pointer, `*mut ()`/`void *`)              |

### Command

Each specific Command instance (combination of Driver and Command number) must
specify its argument type, failure type, and success type. If a Command instance
is invoked with the wrong argument type, the Command call will failure with
error code `INVALID`.

Arguments:

| Register | Argument                    | Type            |
| -------- | --------------------------- | --------------- |
| a1       | Driver number               | `u32`           |
| a2       | Command number              | `u32`           |
| a3-...   | Instance-specific arguments | `ArbitraryData` |

The return variants are specific to the Command instance. On ARM, they MUST fit
into an `ArbitraryData(11)`. On RISC-V, they MUST fit into an
`ArbitraryData(15)`.

Design reasoning: ARM has instructions to push and pop multiple registers, so
clobbering all 11 registers is relatively inexpensive. On RISC-V, this clobbers
only caller-saved registers, so the compiler should have the context it needs to
only save registers that are currently in use.

### Allows

The Allow system calls (read-only, read-write, and userspace-readable) are all
the same at the ABI level. Their arguments are return variants are:

Arguments:

| Register | Argument      | Type                 |
| -------- | ------------- | -------------------- |
| a1       | Driver number | `u32`                |
| a2       | Allow number  | `u32`                |
| a3       | Address       | `*mut u8`/`uint8_t*` |
| a4       | Size          | `usize`/`size_t`     |

Return variants:

| Outcome | Type                                                          |
| ------- | ------------------------------------------------------------- |
| Failure | `(ErrorCode, *mut u8, usize)`/`(ErrorCode, uint8_t*, size_t)` |
| Success | `(*mut u8, usize)`/`(uint8_t*, size_t)`                       |

### Memop

If a Memop operation is invoked with the wrong argument type, the operation will
failure with error code `INVALID`.

Arguments:

| Register | Argument           | Type            |
| -------- | ------------------ | --------------- |
| a1       | Operation          | `u32`           |
| a2-...   | Operation argument | `ArbitraryData` |

Operations:

| ID  | Description                                        | Argument | Success  | Failure     |
| --- | -------------------------------------------------- | -------- | -------- | ----------- |
| 0   | Break                                              | `void *` | `()`     | `ErrorCode` |
| 1   | SBreak                                             | `isize`  | `void *` | `ErrorCode` |
| 2   | Get process RAM start address                      | `()`     | `void *` | `ErrorCode` |
| 3   | Get process RAM allocation length                  | `()`     | `usize`  | `ErrorCode` |
| 4   | Get process flash start address                    | `()`     | `void *` | `ErrorCode` |
| 5   | Get process flash region length                    | `()`     | `usize`  | `ErrorCode` |
| 6   | Get lowest address (end) of the grant region       | `()`     | `usize`? | `ErrorCode` |
| 7   | Get num. writeable flash regions in process header | `()`     | `u32`    | `ErrorCode` |
| 8   | Get start address of a writeable flash region      | `u32`    | `void *` | `ErrorCode` |
| 9   | Get length of a writeable flash region             | `u32`    | `usize`  | `ErrorCode` |
| 10  | Set the start of the process stack                 | `usize`  | `()`     | `ErrorCode` |
| 11  | Set the start of the process heap                  | `usize`  | `()`     | `ErrorCode` |

Note: there are several places where pointer versus usize can be bikeshed. Also,
I did not fully adapt this for CHERI.

### Exit

Arguments:

| Register | Argument        | Type  |
| -------- | --------------- | ----- |
| a1       | Exit number     | `u32` |
| a2       | Completing code | `u32` |

`exit-restart` and `exit-terminate` never return so return variants are not
specified.

## Userspace library implementation.

I believe it is possible to use Rust's generics to provide nice APIs for
invoking the above commands. For example, I believe `libtock-rs` could define
a command function with the following signature:

```rust
// Implemented on any type that can be represented as an ArbitraryData(15)
trait CommandReturn { ... }

// Implemented on any type that can be represented as an ArbitraryData in a3-...
trait CommandArgs { ... }

fn command<Args: CommandArgs, Success: CommandReturn, Failure: CommandReturn>(
    driver_num: u32,
    command_num: u32,
    args: Args) -> Result<Result<Success, Failure>, ErrorCode> { ... }
```

For libtock-c and other languages with less powerful generics, we would generate
the interface code for each syscall driver.

It is expected that userspace libraries will provide a way to register upcalls
that have a function signature matching the upcall's concrete type instead of
`ArbitraryData`; they'll wrap that function in one that decodes the
`ArbitraryData`. The userspace library will pass the latter function to
Subscribe.

## Properties

This pros and cons list is specified relative to Tock 2.0's syscall ABI.

Pros:

1. No more manually casting non-numeric types to a `u32`/`usize` to invoke
   Command or an upcall; any casting necessary will be handled by the core
   syscall layer (or code generator).
1. Type safety: If there is a type mismatch between what userspace expects a
   syscall to use and what the syscall actually uses, that will be caught
   (by the kernel for arguments and by the userspace library for return values)
   and result in an error code rather than data corruption.
1. Argument count checks: no more `command(..., ..., ..., 0 /* unused */)`;
   syscall invocations specify exactly the number of arguments the system call
   needs. The same applies to upcalls.
1. Command can accept many more arguments and return many more values in both
   its success and error case. An arbitrary collection of useful data types can
   be specified, with clear semantics.

Cons:

1. Command clobbers many registers. The impact of this is somewhat mitigated on
   ARM because it has instructions to push and pop multiple registers. It is
   also somewhat mitigated on RISC-V because it only clobbers caller-saved
   registers (so the compiler has local context on which registers need to be
   saved).
1. The userspace implementation of Yield is now larger, as it has to invoke the
   upcall itself. On some platforms, this may involve passing arguments via the
   stack. I would expect Yield to not be inlined with this ABI, whereas in Tock
   2.0 I would expect it to be inlined.
1. It adds error paths to the kernel, because system calls (particularly
   Command) can fail in a new way (incorrect argument type).
1. Similarly, upcall arguments can error, if the passed `ArbitraryData(4)` does
   not match the type expected by the userspace driver.
1. If an `ArbitraryData` of unexpected type is received, there is no reliable
   mechanism for userspace to detect whether the system calls succeeded or
   failed.
1. Queued upcalls are one register more expensive to store in the kernel
   (`ArbitraryData(4)` costs 4 registers whereas in Tock 2.0 only three `usize`
   arguments are passed).
1. We only have one ID left in the type descriptor table so we may run out in
   the future. Technically that's *okay* because anything can be passed as a
   `Register`, but it would be unfortunate and would lose type safety.

Other design notes and observations:

1. The number of arguments you can pass to Command -- and the number of values
   it can return -- depends on architecture. This is great for out-of-tree use
   cases of Tock on platforms with lots of registers (e.g. 64 bit RISC-V) who
   want to have a very complex system call, but bad for the uniformity of the
   Tock ecosystem. I would expect is to have a rule that *upstreamed* system
   call ABIs be compatible with all architectures (i.e. they never pass more
   arguments or return values than 32-bit ARM can handle).
1. All types (arguments and the set of possible return variants) should be
   constant per-syscall, so type descriptors will never need to be dynamically
   calculated or decoded. Generating an `ArbitraryData` will consist of setting
   the first register to a constant then copying data into the remaining
   registers. Decoding one will consist of comparing the type descriptor to a
   constant (the descriptor of the *expected* type), then copying the data out
   of the remaining registers.
1. The type descriptors are designed so that small tuples (ones with no more
   than 4 types) are specified by 16-bit values. Even though the syscall ABI
   always uses a whole register to pass type descriptors, setting a 16-bit value
   is more efficient than a 32-bit value in some (all?) of the architectures we
   support.
1. We can no longer use `u32`, `usize`, and `*mut ()` interchangeably. In some
   ways this is nice, but it forces us to make decisions that are sometimes
   nonobvious (see e.g. the "get the end of the grant region" memop, which could
   return either `usize` or `*mut ()`).
1. Just a design note: this ABI uses caller-saved registers before callee-saved
   registers in an attempt to minimize the extent to which registers need to be
   pushed onto the stack when making syscalls.

## Possible improvements

As a reminder, the goals of this document are promote the development of a more
type-safe syscall API for Tock and to spread CHERI knowledge, and the above ABI
was designed accordingly. For practical implementation, there are several ways
we could improve on the ABI proposed above:

1. We could microoptimize the register choices better. We can probably make the
   kernel return Upcall arguments in the same registers as they will be passed
   into the callback so userspace's Yield implementation does not need to
   perform a series of register moves. There may also be a better order for the
   Command registers to reduce the amount of register shuffling that happens in
   the userspace libraries and/or kernel.
1. We can trim down the type list some more. The type descriptors are mainly
   useful for parts of the syscall ABI where capsules can specify data types,
   which are Command's arguments, Command's return values, and upcall arguments.
   The system calls with data types that are fully-specified in the syscall ABI
   (Allows, Memop, Subscribe, and Yield) can play looser with types without much
   risk. Therefore, we can probably remove the upcall pointer and data pointer
   types entirely, and replace them with arbitrary register values. If we are
   willing to limit Tock processes to 2 GiB of RAM each, then we can remove
   `isize` and use `i32` for relative addresses instead.
1. We probably want to limit what types Command and upcalls can use. E.g. we
   may want to disallow using pointers as Command arguments or return values.
1. Depending on what types of errors we want to catch, it may not be necessary
   to have type descriptors. If we create a system call definition format and
   autogenerate the system call interface (in both the kernel and userspace),
   then (assuming the code generator is correct) type mismatches become
   impossible. That would free up a register, remove some runtime checks, and
   allow for more than 15 types. The downside is that type mismatches could
   still happen if a user runs an app on a kernel that was compiled for a
   different version of the syscall definition. We could mitigate that by having
   CI checks that prevent ABI-breaking changes to syscall interfaces.

## Discussion

The [PR that added this
document](https://github.com/tock/design-explorations/pull/4) has good
discussion on this topic.
