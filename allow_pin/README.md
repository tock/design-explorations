# `Pin`-based Allow API size comparison

Original author: Johnathan Van Why

There are a lot of design decisions to be made while implementing an Allow API:

1. Do you unconditionally un-allow the buffer in drop(), or do you use a
   variable to track whether the buffer is currently shared so that the drop()
   unallow can perhaps be optimized away?
2. Do you have separate types for RO allow or RW allow, or a single type that
   can do both (interacts with the above, as one variable can track the
   unshared versus share type status)?
3. Do you make DRIVER_NUM and BUFFER_NUM const or variables?

To support making those decisions, this directory contains code size
benchmarking infrastructure. The library contains a couple different Allow API
implementations, and examples/ contains several code examples (each one ported
to each Allow API). The Makefile compiles all the examples at several
optimization levels, disassembles them (dumping the disassemblies into
disassembly/) and performs size measurements (producing arm_sizes and
riscv_sizes).

The current API implementations are:

1. `no_dynamic` -- There is no variable tracking whether a buffer is shared. Any
   operation that needs the buffer unshared (such as reading the buffer)
   unconditionally unallows the buffer. This results in duplicate and
   unnecessary unallows, and some unpleasant behavior (such as an allow buffer
   that was never shared with the kernel still calling unallow at the end of
   scope).
2. `dynamic_type` -- This tracks whether the buffer is shared at runtime, and is
   a single type that can perform both RO and RW allows. However, the allow
   number is still `const`, so it's not as dynamic as possible.
3. `full_dynamic` -- This tracks whether the buffer is shared, whether it is
   shared read-only or read-write, and the allow ID at runtime, making it the
   most dynamic option possible.

Currently, `dynamic_type` seems more expensive than the other options, while
`no_dynamic` and `full_dynamic` have similar code sizes to each other (for the
large complex example).
