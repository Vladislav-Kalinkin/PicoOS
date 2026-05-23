use crate::arch;
use crate::arch::aarch64::cpu;
use crate::drivers::uart;
use crate::kernel::trap_frame::Aarch64TrapFrame;

#[no_mangle]
pub extern "C" fn sync_exception_current_el_sp0() -> ! {
    print_exception("SYNC EXCEPTION: current EL, SP0");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn irq_current_el_sp0() -> ! {
    print_exception("IRQ: current EL, SP0");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn fiq_current_el_sp0() -> ! {
    print_exception("FIQ: current EL, SP0");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn serror_current_el_sp0() -> ! {
    print_exception("SError: current EL, SP0");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn sync_exception_current_el_spx() -> ! {
    print_exception("SYNC EXCEPTION: current EL, SPx");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn irq_current_el_spx(frame: *const Aarch64TrapFrame) {
    crate::handle_irq(frame);
}

#[no_mangle]
pub extern "C" fn fiq_current_el_spx() -> ! {
    print_exception("FIQ: current EL, SPx");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn serror_current_el_spx() -> ! {
    print_exception("SError: current EL, SPx");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn sync_exception_lower_el_aarch64() -> ! {
    print_exception("SYNC EXCEPTION: lower EL AArch64");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn irq_lower_el_aarch64() -> ! {
    print_exception("IRQ: lower EL AArch64");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn fiq_lower_el_aarch64() -> ! {
    print_exception("FIQ: lower EL AArch64");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn serror_lower_el_aarch64() -> ! {
    print_exception("SError: lower EL AArch64");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn sync_exception_lower_el_aarch32() -> ! {
    print_exception("SYNC EXCEPTION: lower EL AArch32");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn irq_lower_el_aarch32() -> ! {
    print_exception("IRQ: lower EL AArch32");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn fiq_lower_el_aarch32() -> ! {
    print_exception("FIQ: lower EL AArch32");
    arch::halt();
}

#[no_mangle]
pub extern "C" fn serror_lower_el_aarch32() -> ! {
    print_exception("SError: lower EL AArch32");
    arch::halt();
}

fn print_exception(name: &str) {
    uart::write_line("");
    uart::write_line("=== EXCEPTION ===");
    uart::write_line(name);

    uart::write_str("ESR_EL1: ");
    uart::write_hex_u64(cpu::esr_el1());
    uart::write_line("");

    uart::write_str("ELR_EL1: ");
    uart::write_hex_u64(cpu::elr_el1());
    uart::write_line("");

    uart::write_str("FAR_EL1: ");
    uart::write_hex_u64(cpu::far_el1());
    uart::write_line("");

    uart::write_str("SPSR_EL1: ");
    uart::write_hex_u64(cpu::spsr_el1());
    uart::write_line("");
}
