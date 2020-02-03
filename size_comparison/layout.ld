/* Layout file for Tock applications that use libtock-rs. This currently
 * implements static linking, because we do not have a working relocation
 * solution for Rust apps. This layout works for all platforms libtock-rs
 * supports (e.g. both ARM and RISC-V apps).
 *
 * FLASH refers to the area the application is loaded in flash, including TBF
 * heeaders. RAM refers to the area the application will have access to in
 * memory. STACK_SIZE is the size of the application's stack. TBF_HEADER_SIZE
 * must correspond to elf2tab's --protected-region-size flag.
 *
 * This places the RAM sections in the following order:
 *     1. Stack -- The stack grows downwards so putting it first gives us
 *                 MPU-based overflow detection.
 *     2. .data -- Read-write global variables.
 *     3. .bss  -- Zero-initialized read-write global variables.
 */

MEMORY {
    FLASH (X) : ORIGIN = 0x76000, LENGTH = 0x10000
    RAM   (W) : ORIGIN = 0x14000, LENGTH = 0x0c000
}

TBF_HEADER_SIZE = 0x40;
STACK_SIZE = 2048;

SECTIONS {
    /* Sections located in FLASH at runtime.
     */

    /* Add a section where elf2tab will place the TBF headers, so that the rest
     * of the FLASH sections are in the right locations. */
    .tbf_header (NOLOAD) : {
        . = . + TBF_HEADER_SIZE;
    } > FLASH

    /* Runtime header. Contains values the linker knows that the runtime needs
     * to look up.
     */
    .rt_header : {
        rt_header = .;
        LONG(SIZEOF(.data));       /* Size of .data section */
        LONG(ADDR(.bss));          /* Address of .bss in RAM */
        LONG(SIZEOF(.bss));        /* Size of .bss section */
    } > FLASH

    /* Text section -- the application's code. */
    .text ALIGN(4) : {
        /* `start` is a special symbol name the linker uses to identify the
         * entry point. */
        start = .;
        KEEP(*(.start))
        *(.text)
    } > FLASH

    /* Read-only data section. Contains strings and other global constants. */
    .rodata ALIGN(4) : {
        *(.rodata)
        /* .data is placed after .rodata in flash. data_flash_start is used by
         * AT() to place .data in flash and by the entry point to find .data.
         */
        data_flash_start = .;
    } > FLASH

    /* Sections located in RAM at runtime.
     */

    /* Reserve space for the stack. Aligned to a multiple of 8 bytes to follow
     * the ARM AAPCS:
     * http://infocenter.arm.com/help/index.jsp?topic=/com.arm.doc.faqs/ka4127.html
     */
    .stack ALIGN(8) (NOLOAD) : {
        . = . + STACK_SIZE;
        stack_top = .;  /* Used by the entry point */
    } > RAM

    /* Read-write data section. This deployed as part of FLASH but is copied
     * into RAM at runtime.
     */
    .data ALIGN(4) : AT(data_flash_start) {
        data_ram_start = .;
        *(.data)
    } > RAM

    /* BSS section. These are zero-initialized static variables. This section is
     * not copied from FLASH into RAM but rather directly initialized, and is
     * mainly put in this linker script so that we get an error if it overflows
     * the RAM region.
     */
    .bss ALIGN(4) (NOLOAD) : {
        *(.bss)
    } > RAM

    heap_start = ADDR(.bss) + SIZEOF(.bss);  /* Used by the entry point */

    /* Discard unwinding tables */
    /DISCARD/ : {
        *(.ARM.exidx)
    } > FLASH
}
