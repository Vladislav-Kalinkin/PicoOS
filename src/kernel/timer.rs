use core::arch::asm;

pub fn frequency_hz() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, CNTFRQ_EL0",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

pub fn counter() -> u64 {
    let value: u64;

    unsafe {
        asm!(
            "mrs {0}, CNTPCT_EL0",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    value
}

#[allow(dead_code)]
pub fn delay_ticks(ticks: u64) {
    let start = counter();

    loop {
        let now = counter();

        if now.wrapping_sub(start) >= ticks {
            break;
        }
    }
}

#[allow(dead_code)]
pub fn delay_ms(ms: u64) {
    let freq = frequency_hz();
    let ticks = (freq / 1000) * ms;

    delay_ticks(ticks);
}

#[allow(dead_code)]
pub fn delay_seconds(seconds: u64) {
    let freq = frequency_hz();

    delay_ticks(freq * seconds);
}

pub fn set_timer_after_ticks(ticks: u32) {
    unsafe {
        asm!(
            "msr CNTP_TVAL_EL0, {0:x}",
            in(reg) ticks,
            options(nomem, nostack, preserves_flags)
        );
    }
}

pub fn enable_timer_irq() {
    unsafe {
        asm!(
            "msr CNTP_CTL_EL0, {0:x}",
            in(reg) 1u64,
            options(nomem, nostack, preserves_flags)
        );
    }
}

pub fn disable_timer() {
    unsafe {
        asm!(
            "msr CNTP_CTL_EL0, {0:x}",
            in(reg) 0u64,
            options(nomem, nostack, preserves_flags)
        );
    }
}

pub fn arm_timer_ticks(ticks: u64) {
    set_timer_after_ticks(ticks as u32);
    enable_timer_irq();
}

pub fn arm_timer_hz(hz: u64) {
    let freq = frequency_hz();
    let ticks = freq / hz;

    arm_timer_ticks(ticks);
}

#[allow(dead_code)]
pub fn arm_timer_seconds(seconds: u64) {
    let freq = frequency_hz();
    let ticks = freq * seconds;

    arm_timer_ticks(ticks);
}
