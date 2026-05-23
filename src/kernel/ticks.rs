use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

pub const MAX_TEST_TICKS: u64 = 5;

pub fn reset() {
    TICKS.store(0, Ordering::SeqCst);
}

pub fn increment() -> u64 {
    TICKS.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn get() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

pub fn is_test_complete() -> bool {
    get() >= MAX_TEST_TICKS
}
