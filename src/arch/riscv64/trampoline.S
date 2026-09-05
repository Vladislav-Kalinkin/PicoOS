.section .usertext, "ax", @progbits
.align 4
.global user_trampoline
.type user_trampoline, @function
user_trampoline:
    /* a0 = entry in .usertext, a1 = spawn arg */
    mv   t0, a0
    mv   a0, a1
    jalr t0
    /* entry returned — treat as exit */
    li   a7, 2
    .option norvc
    ecall
