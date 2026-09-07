use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::platform::HART_SLOTS;

/// Per-hart cell. Not a multi-hart mutex.
///
/// Index by `mhartid`. Each hart touches only `slots[hart]`. IRQs remain off
/// in M while in the kernel on that hart (`without_interrupts`).
pub struct HartLocal<T> {
    slots: [UnsafeCell<T>; HART_SLOTS],
}

// SAFETY: this hart indexes only `slots[mhartid]`; overflow harts never leave
// `boot.S`; mutation happens only while MIE is clear on this hart.
unsafe impl<T: Send> Sync for HartLocal<T> {}

impl<T: Copy> HartLocal<T> {
    pub const fn new(value: T) -> Self {
        const { assert!(HART_SLOTS == 8) };
        Self {
            slots: [
                UnsafeCell::new(value),
                UnsafeCell::new(value),
                UnsafeCell::new(value),
                UnsafeCell::new(value),
                UnsafeCell::new(value),
                UnsafeCell::new(value),
                UnsafeCell::new(value),
                UnsafeCell::new(value),
            ],
        }
    }
}

impl<T> HartLocal<T> {
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let hart = hart_index();
        crate::arch::without_interrupts(|| {
            // SAFETY: IRQs off; this hart is the only accessor of `slots[hart]`.
            f(unsafe { &mut *self.slots[hart].get() })
        })
    }

    /// Access raw slot pointer by hart index.
    ///
    /// # Safety
    /// Caller must ensure that accessing slot `hart` does not violate single-hart mutability
    /// invariants (e.g. only during hart 0 early boot setup or read-only metric collection).
    pub unsafe fn slot_ptr(&self, hart: usize) -> *mut T {
        if hart >= HART_SLOTS {
            crate::arch::halt();
        }
        self.slots[hart].get()
    }
}

pub fn hart_index() -> usize {
    let hart = crate::arch::riscv64::cpu::mhartid() as usize;
    if hart >= HART_SLOTS {
        crate::arch::halt();
    }
    hart
}

/// Who is in U on each hart. **0 = idle**, never a valid Frame id after PR5
/// (`generation` starts at 1). Publication, not a lock.
static HART_U_TID: [AtomicU64; HART_SLOTS] = [const { AtomicU64::new(0) }; HART_SLOTS];

pub fn publish_u_tid(tid: u64) {
    HART_U_TID[hart_index()].store(tid, Ordering::Release);
}

pub fn clear_u_tid() {
    HART_U_TID[hart_index()].store(0, Ordering::Release);
}
