use crate::drivers::mmio;
use crate::platform;

pub fn mtime() -> u64 {
    mmio::read64(platform::CLINT_MTIME)
}

pub fn set_mtimecmp(value: u64) {
    mmio::write64(platform::CLINT_MTIMECMP, value);
}

pub fn timebase_frequency() -> u64 {
    platform::TIMEBASE_FREQ
}

pub fn arm_timer_after_ticks(ticks: u64) {
    let now = mtime();
    set_mtimecmp(now.wrapping_add(ticks));
}

pub fn arm_timer_hz(hz: u64) {
    let ticks = timebase_frequency() / hz;
    arm_timer_after_ticks(ticks);
}

#[allow(dead_code)]
pub fn arm_timer_seconds(seconds: u64) {
    arm_timer_after_ticks(timebase_frequency() * seconds);
}

pub fn disarm_timer() {
    set_mtimecmp(u64::MAX);
}
