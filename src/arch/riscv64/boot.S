.section .text.boot
.global _start
.type _start, @function

/* Must match platform::HART_SLOTS. Overflow harts never index HART_GO. */
.equ HART_SLOTS, 8

_start:
    csrr t0, mhartid
    li   t1, HART_SLOTS
    bgeu t0, t1, hart_overflow
    bnez t0, hart_wait

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

hart_overflow:
    wfi
    j hart_overflow

hart_wait:
    /* t0 = mhartid. Busy-load GO; do not wfi (mie.MSIE is still 0). */
    la t1, HART_PRESENT
    add t1, t1, t0
    li t2, 1
    sb t2, 0(t1)
    la t1, HART_GO
    add t1, t1, t0
    lbu t2, 0(t1)
    beqz t2, hart_wait
    /* PR1: no secondary bring-up. GO is never written; if it were, hang. */
    j hang
