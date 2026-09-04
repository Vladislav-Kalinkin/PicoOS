.section .text.trap
.align 8
.global trap_vector

trap_vector:
    csrrw sp, mscratch, sp
    addi sp, sp, -232

    sd t0, 16(sp)
    csrr t0, mscratch
    sd t0, 0(sp)

    sd ra, 8(sp)
    sd t1, 24(sp)
    sd t2, 32(sp)
    sd t3, 40(sp)
    sd t4, 48(sp)
    sd t5, 56(sp)
    sd t6, 64(sp)

    sd a0, 72(sp)
    sd a1, 80(sp)
    sd a2, 88(sp)
    sd a3, 96(sp)
    sd a4, 104(sp)
    sd a5, 112(sp)
    sd a6, 120(sp)
    sd a7, 128(sp)

    sd s0, 136(sp)
    sd s1, 144(sp)
    sd s2, 152(sp)
    sd s3, 160(sp)
    sd s4, 168(sp)
    sd s5, 176(sp)
    sd s6, 184(sp)
    sd s7, 192(sp)
    sd s8, 200(sp)
    sd s9, 208(sp)
    sd s10, 216(sp)
    sd s11, 224(sp)

    mv a0, sp
    call riscv64_trap_handler

    la t6, __trap_stack_top
    csrw mscratch, t6

    ld ra, 8(sp)
    ld t0, 16(sp)
    ld t1, 24(sp)
    ld t2, 32(sp)
    ld t3, 40(sp)
    ld t4, 48(sp)
    ld t5, 56(sp)
    ld t6, 64(sp)

    ld a0, 72(sp)
    ld a1, 80(sp)
    ld a2, 88(sp)
    ld a3, 96(sp)
    ld a4, 104(sp)
    ld a5, 112(sp)
    ld a6, 120(sp)
    ld a7, 128(sp)

    ld s0, 136(sp)
    ld s1, 144(sp)
    ld s2, 152(sp)
    ld s3, 160(sp)
    ld s4, 168(sp)
    ld s5, 176(sp)
    ld s6, 184(sp)
    ld s7, 192(sp)
    ld s8, 200(sp)
    ld s9, 208(sp)
    ld s10, 216(sp)
    ld s11, 224(sp)

    ld sp, 0(sp)

    mret
