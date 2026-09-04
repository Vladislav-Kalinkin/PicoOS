use crate::drivers::uart;

pub fn print_boot_banner() {
    uart::write_line("================================");
    uart::write_line("PicoOS 0.2.0");
    uart::write_line("Frame Kernel");
    uart::write_line("================================");
}

pub fn print_capabilities() {
    uart::write_line("");
    uart::write_line("kernel capabilities:");
    uart::write_line("- architecture: riscv64");
    uart::write_line("- UART console");
    uart::write_line("- U-mode frames");
    uart::write_line("- PMP deny (store to .data faults the task)");
    uart::write_line("- ecall yield/sleep/exit/log");
    uart::write_line("- timer preemption via mret");
    uart::write_line("- page allocator with free and reap");
    uart::write_line("- contract-checked resume");
}
