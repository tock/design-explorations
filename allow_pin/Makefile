# The architecture names (arm, riscv), target names (thumbv6m-none-eabi,
# riscv32imc-unknown-none-elf), and binutils prefixes (arm-none-eabi- and
# riscv64-unknown-elf-) are all different, so a lot of things need to be
# duplicated between the architectures.

# Hacky way to get the names of the example binaries.
examples ::= $(patsubst examples/%.rs,%,$(wildcard examples/*.rs))
opt_levels ::= 1 2 3 s z

# ELF files created by cargo for the examples.
arm_elfs ::= $(foreach example,$(examples),$(patsubst %,target/thumbv6m-none-eabi/opt-%/examples/$(example),$(opt_levels)))
riscv_elfs ::= $(foreach example,$(examples),$(patsubst %,target/riscv32imc-unknown-none-elf/opt-%/examples/$(example),$(opt_levels)))

# All disassembly report targets.
disassemblies_arm ::= $(foreach level,$(opt_levels),$(patsubst %,disassembly/%-arm-opt-$(level),$(examples)))
disassemblies_riscv ::= $(foreach level,$(opt_levels),$(patsubst %,disassembly/%-riscv-opt-$(level),$(examples)))

.PHONY: all clean $(arm_elfs) $(riscv_elfs)
all: arm_sizes $(disassemblies_arm) $(disassemblies_riscv) riscv_sizes

clean:
	cargo clean
	rm -rf arm_sizes riscv_sizes disassembly/

# Actions to compile the examples using cargo.
define cargo_target
$(patsubst %,target/$(target)/opt-$(level)/examples/%,$(examples)) &:
	cargo build --examples --profile opt-$(level) --target $(target)
endef # cargo_target
$(foreach target,thumbv6m-none-eabi riscv32imc-unknown-none-elf,$(foreach level,$(opt_levels),$(eval $(cargo_target))))

# Creates the folder the disassembly will be written into.
disassembly:
	mkdir -p disassembly

# Actions to create the disassembly files for the disassembly/ directory.
define arm_disassembly_target
disassembly/$(example)-arm-opt-$(level): disassembly target/thumbv6m-none-eabi/opt-$(level)/examples/$(example)
	arm-none-eabi-objdump -hd target/thumbv6m-none-eabi/opt-$(level)/examples/$(example) \
		>disassembly/$(example)-arm-opt-$(level)
endef # arm_disassembly_target
$(foreach level,$(opt_levels),$(foreach example,$(examples),$(eval $(arm_disassembly_target))))
define riscv_disassembly_target
disassembly/$(example)-riscv-opt-$(level): disassembly target/riscv32imc-unknown-none-elf/opt-$(level)/examples/$(example)
	riscv64-unknown-elf-objdump -hd target/riscv32imc-unknown-none-elf/opt-$(level)/examples/$(example) \
		>disassembly/$(example)-riscv-opt-$(level)
endef # arm_disassembly_target
$(foreach level,$(opt_levels),$(foreach example,$(examples),$(eval $(riscv_disassembly_target))))

# Actions to create the size reports
arm_sizes: $(arm_elfs)
	arm-none-eabi-size $(arm_elfs) > arm_sizes
riscv_sizes: $(riscv_elfs)
	riscv64-unknown-elf-size $(riscv_elfs) > riscv_sizes
