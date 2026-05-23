ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    __kernel_start = .;

    .text : ALIGN(4K) {
        __text_start = .;
        KEEP(*(.text.boot))
        KEEP(*(.text.trap))
        *(.text*)
        __text_end = .;
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

    . = ALIGN(16);
    . += 0x10000;
    __stack_top = .;

    . = ALIGN(4K);
    __kernel_end = .;
    __free_memory_start = .;
}
