.section .text.boot
.global _start
.type _start, %function

_start:
    ldr x0, =__stack_top
    mov sp, x0

    ldr x0, =__bss_start
    ldr x1, =__bss_end

clear_bss_loop:
    cmp x0, x1
    b.ge clear_bss_done
    str xzr, [x0], #8
    b clear_bss_loop

clear_bss_done:
    bl kernel_main

hang:
    nop
    b hang
