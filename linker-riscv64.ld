ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    __kernel_start = .;

    .text : ALIGN(4K) {
        __text_start = .;
        KEEP(*(.text.boot))
        KEEP(*(.text.trap))
        *(.text .text.*)
        __text_end = .;
    }

    .usertext : ALIGN(4K) {
        __user_text_start = .;
        . += 4;
        KEEP(*(.usertext .usertext.*))
        . = ALIGN(4);
        __user_text_end = .;
    }

    .rodata : ALIGN(4K) {
        __rodata_start = .;
        *(.rodata*)
        __rodata_end = .;
    }

    .data : ALIGN(4K) {
        __data_start = .;
        *(.data*)
        __data_end = .;
    }

    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
    }

    /* 64 KiB kernel stack, 64 KiB-aligned so a later TOR/NAPOT can cover it. */
    . = ALIGN(64K);
    . += 0x10000;
    __stack_top = .;

    . = ALIGN(4K);
    __kernel_end = .;

    /* Own 4 KiB section after the kernel image, not swallowed by a .bss TOR. */
    .trap_stack (NOLOAD) : ALIGN(4K) {
        __trap_stack_bottom = .;
        . += 4096;
        __trap_stack_top = .;
    }

    . = ALIGN(4K);
    __free_memory_start = .;
}
