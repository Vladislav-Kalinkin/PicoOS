use core::cell::UnsafeCell;

/// Uniprocessor shared cell. Access only through [`IrqCell::with`], which
/// runs the closure inside [`crate::arch::without_interrupts`].
pub struct IrqCell<T> {
    inner: UnsafeCell<T>,
}

// SAFETY: one hart; mutation happens only while MIE is clear.
unsafe impl<T> Sync for IrqCell<T> {}

impl<T> IrqCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        crate::arch::without_interrupts(|| {
            // SAFETY: IRQs off: yes. Pointer: the cell is 'static and this
            // hart is the only accessor while MIE is clear.
            f(unsafe { &mut *self.inner.get() })
        })
    }
}
