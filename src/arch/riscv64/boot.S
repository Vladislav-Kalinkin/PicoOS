.section .text.boot
.global _start
.type _start, @function

_start:
    la sp, __stack_top

    la t0, __bss_start
    la t1, __bss_end

clear_bss_loop:
    bgeu t0, t1, clear_bss_done
    sd zero, 0(t0)
    addi t0, t0, 8
    j clear_bss_loop

clear_bss_done:
    la t0, __trap_stack_bottom
    la t1, __trap_stack_top

clear_trap_stack_loop:
    bgeu t0, t1, clear_trap_stack_done
    sd zero, 0(t0)
    addi t0, t0, 8
    j clear_trap_stack_loop

clear_trap_stack_done:
    call kernel_main

hang:
    wfi
    j hang
