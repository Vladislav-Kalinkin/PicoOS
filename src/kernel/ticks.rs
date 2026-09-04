use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

#[cfg(any(feature = "task_yield_test", feature = "kernel_fault_guard_test"))]
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
    #[cfg(any(feature = "task_yield_test", feature = "kernel_fault_guard_test"))]
    {
        get() >= MAX_TEST_TICKS
    }
    #[cfg(not(any(feature = "task_yield_test", feature = "kernel_fault_guard_test")))]
    {
        false
    }
}
