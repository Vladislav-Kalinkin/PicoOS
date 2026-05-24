use crate::drivers::uart;

pub fn print_boot_banner() {
    uart::write_line("================================");
    uart::write_line("PicoOS 0.1.18");
    uart::write_line("early RISC-V kernel");
    uart::write_line("================================");
}

pub fn print_capabilities() {
    uart::write_line("");
    uart::write_line("kernel capabilities:");
    uart::write_line("- architecture: riscv64");
    uart::write_line("- UART console");
    uart::write_line("- exception/trap handling");
    uart::write_line("- timer interrupts");
    uart::write_line("- page allocator");
    uart::write_line("- kernel heap");
    uart::write_line("- task table");
    uart::write_line("- task stacks");
    uart::write_line("- cooperative task runner skeleton");
    uart::write_line("- selftest mode");
}
