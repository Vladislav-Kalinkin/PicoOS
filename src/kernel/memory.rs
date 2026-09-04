use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;
use crate::platform;

pub const PAGE_SIZE: u64 = 4096;

/// Static bitmap capacity: 512 × 64 bits = 32768 pages = 128 MiB from `RAM_START`.
const BITMAP_WORDS: usize = 512;
const BITMAP_BITS: usize = BITMAP_WORDS * 64;

#[cfg(debug_assertions)]
const FREE_POISON: u8 = 0xA5;

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;

    static __text_start: u8;
    static __text_end: u8;

    static __rodata_start: u8;
    static __rodata_end: u8;

    static __data_start: u8;
    static __data_end: u8;

    static __bss_start: u8;
    static __bss_end: u8;

    static __stack_top: u8;
    static __free_memory_start: u8;
}

/// Physical page base. Address is 4 KiB aligned and in the managed pool
/// after a successful [`alloc_pages`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PhysPage {
    addr: u64,
}

impl PhysPage {
    pub const fn addr(self) -> u64 {
        self.addr
    }

    pub const fn new(addr: u64) -> Option<Self> {
        if addr.is_multiple_of(PAGE_SIZE) {
            Some(Self { addr })
        } else {
            None
        }
    }
}

/// Page counts for the managed pool. The bitmap lives in `.bss` and is not
/// included in `used`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MmStats {
    pub free: u64,
    pub used: u64,
    pub high_water: u64,
}

struct MmState {
    bitmap: [u64; BITMAP_WORDS],
    base: u64,
    nbits: usize,
    used: u64,
    high_water: u64,
}

static MM: IrqCell<MmState> = IrqCell::new(MmState {
    bitmap: [0; BITMAP_WORDS],
    base: 0,
    nbits: 0,
    used: 0,
    high_water: 0,
});

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

pub fn init() {
    let ram_end = (platform::RAM_START as u64).checked_add(platform::RAM_SIZE as u64);
    let start = align_up(free_memory_start(), PAGE_SIZE);

    MM.with(|mm| {
        *mm = MmState {
            bitmap: [0; BITMAP_WORDS],
            base: 0,
            nbits: 0,
            used: 0,
            high_water: 0,
        };

        let (Some(start), Some(ram_end)) = (start, ram_end) else {
            return;
        };
        if start >= ram_end {
            return;
        }

        let span = ram_end - start;
        let mut nbits = (span / PAGE_SIZE) as usize;
        if nbits > BITMAP_BITS {
            nbits = BITMAP_BITS;
        }

        // Bits [nbits, BITMAP_BITS) stay stuck invalid (not free).
        for bit in nbits..BITMAP_BITS {
            set_bit(&mut mm.bitmap, bit);
        }

        mm.base = start;
        mm.nbits = nbits;
    });
}

/// Allocate `n` consecutive pages. First-fit. Returns `None` on OOM, `n == 0`,
/// or if the allocator is not initialized.
///
/// Forbidden on the IRQ/trap prologue path. Spawn and later reap run with
/// IRQs already off on the kernel stack, not from a nested trap.
pub fn alloc_pages(n: usize) -> Option<PhysPage> {
    if n == 0 {
        return None;
    }

    MM.with(|mm| {
        if mm.base == 0 || mm.nbits == 0 || n > mm.nbits {
            return None;
        }

        let start = find_free_run(&mm.bitmap, mm.nbits, n)?;
        for bit in start..start + n {
            set_bit(&mut mm.bitmap, bit);
        }

        mm.used += n as u64;
        if mm.used > mm.high_water {
            mm.high_water = mm.used;
        }

        Some(PhysPage {
            addr: mm.base + (start as u64) * PAGE_SIZE,
        })
    })
}

/// Free `n` consecutive pages previously returned by [`alloc_pages`].
/// Invalid ranges (unaligned, out of pool, not fully allocated) are ignored.
///
/// Forbidden on the IRQ/trap prologue path.
pub fn free_pages(base: PhysPage, n: usize) {
    if n == 0 {
        return;
    }

    MM.with(|mm| {
        let Some(start) = page_bit(mm, base.addr) else {
            return;
        };
        let Some(end) = start.checked_add(n) else {
            return;
        };
        if end > mm.nbits {
            return;
        }
        for bit in start..end {
            if !test_bit(&mm.bitmap, bit) {
                return;
            }
        }

        // Poison while the bits are still allocated so a concurrent alloc
        // cannot observe the page mid-wipe. Uniprocessor + IrqCell.
        #[cfg(debug_assertions)]
        poison_pages(base.addr, n);

        for bit in start..end {
            clear_bit(&mut mm.bitmap, bit);
        }
        mm.used = mm.used.saturating_sub(n as u64);
    });
}

pub fn stats() -> MmStats {
    MM.with(|mm| {
        let free = (mm.nbits as u64).saturating_sub(mm.used);
        MmStats {
            free,
            used: mm.used,
            high_water: mm.high_water,
        }
    })
}

pub fn allocate_page() -> Option<u64> {
    alloc_pages(1).map(PhysPage::addr)
}

pub fn free_memory_current() -> u64 {
    MM.with(|mm| {
        if mm.base == 0 || mm.nbits == 0 {
            return 0;
        }
        match find_free_run(&mm.bitmap, mm.nbits, 1) {
            Some(bit) => mm.base + (bit as u64) * PAGE_SIZE,
            None => 0,
        }
    })
}

pub fn memory_end() -> u64 {
    MM.with(|mm| {
        if mm.base == 0 {
            0
        } else {
            mm.base + (mm.nbits as u64) * PAGE_SIZE
        }
    })
}

pub fn kernel_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__kernel_start))
}

pub fn kernel_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__kernel_end))
}

pub fn text_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__text_start))
}

pub fn text_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__text_end))
}

pub fn rodata_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__rodata_start))
}

pub fn rodata_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__rodata_end))
}

pub fn data_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__data_start))
}

pub fn data_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__data_end))
}

pub fn bss_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__bss_start))
}

pub fn bss_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__bss_end))
}

pub fn stack_top() -> u64 {
    symbol_addr(core::ptr::addr_of!(__stack_top))
}

pub fn free_memory_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__free_memory_start))
}

pub fn kernel_text_start() -> u64 {
    text_start()
}

pub fn kernel_text_end() -> u64 {
    text_end()
}

pub fn is_inside_kernel_text(addr: u64) -> bool {
    addr >= kernel_text_start() && addr < kernel_text_end()
}

pub fn print_memory_layout() {
    uart::write_line("");
    uart::write_line("memory layout:");

    print_range("kernel", kernel_start(), kernel_end());
    print_range("text", text_start(), text_end());
    print_range("rodata", rodata_start(), rodata_end());
    print_range("data", data_start(), data_end());
    print_range("bss", bss_start(), bss_end());

    uart::write_str("stack_top: ");
    uart::write_hex_u64(stack_top());
    uart::write_line("");

    uart::write_str("free_memory_start: ");
    uart::write_hex_u64(free_memory_start());
    uart::write_line("");

    uart::write_str("RAM start: ");
    uart::write_hex_u64(platform::RAM_START as u64);
    uart::write_line("");

    uart::write_str("RAM end: ");
    uart::write_hex_u64(platform::RAM_START as u64 + platform::RAM_SIZE as u64);
    uart::write_line("");
}

pub fn test_page_allocator() {
    uart::write_line("");
    uart::write_line("page allocator:");

    init();

    uart::write_str("page size: ");
    uart::write_dec_u64(PAGE_SIZE);
    uart::write_line(" bytes");

    uart::write_str("RAM start: ");
    uart::write_hex_u64(platform::RAM_START as u64);
    uart::write_line("");

    uart::write_str("RAM end: ");
    uart::write_hex_u64(memory_end());
    uart::write_line("");

    uart::write_str("initial free page: ");
    uart::write_hex_u64(free_memory_current());
    uart::write_line("");

    let first = allocate_and_print();
    allocate_and_print();
    allocate_and_print();

    uart::write_str("next free page: ");
    uart::write_hex_u64(free_memory_current());
    uart::write_line("");

    print_mm_stats();

    let used_after_alloc = stats().used;
    if used_after_alloc != 3 {
        uart::write_line("page free+realloc: FAILED used after alloc");
        return;
    }

    if let Some(page) = first.and_then(PhysPage::new) {
        free_pages(page, 3);
    } else {
        uart::write_line("page free+realloc: FAILED missing first page");
        return;
    }

    if stats().used != 0 {
        uart::write_line("page free+realloc: FAILED used after free");
        return;
    }

    let Some(again) = alloc_pages(3) else {
        uart::write_line("page free+realloc: FAILED realloc");
        return;
    };

    uart::write_str("reallocated page: ");
    uart::write_hex_u64(again.addr());
    uart::write_line("");

    let ok = first == Some(again.addr()) && stats().used == 3;
    free_pages(again, 3);

    if ok && stats().used == 0 {
        uart::write_line("page free+realloc: OK");
    } else {
        uart::write_line("page free+realloc: FAILED");
    }

    print_mm_stats();
}

pub fn print_mm_stats() {
    let mm = stats();
    uart::write_str("mm free: ");
    uart::write_dec_u64(mm.free);
    uart::write_line("");
    uart::write_str("mm used: ");
    uart::write_dec_u64(mm.used);
    uart::write_line("");
    uart::write_str("mm high_water: ");
    uart::write_dec_u64(mm.high_water);
    uart::write_line("");
}

fn allocate_and_print() -> Option<u64> {
    match allocate_page() {
        Some(page) => {
            uart::write_str("allocated page: ");
            uart::write_hex_u64(page);
            uart::write_line("");
            Some(page)
        }
        None => {
            uart::write_line("allocated page: FAILED");
            None
        }
    }
}

fn print_range(name: &str, start: u64, end: u64) {
    uart::write_str(name);
    uart::write_str(": ");
    uart::write_hex_u64(start);
    uart::write_str(" - ");
    uart::write_hex_u64(end);
    uart::write_str(" size: ");
    uart::write_dec_u64(end.saturating_sub(start));
    uart::write_line(" bytes");
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }

    Some(value.checked_add(align - 1)? & !(align - 1))
}

fn test_bit(bitmap: &[u64; BITMAP_WORDS], bit: usize) -> bool {
    let word = bit / 64;
    let mask = 1u64 << (bit % 64);
    bitmap[word] & mask != 0
}

fn set_bit(bitmap: &mut [u64; BITMAP_WORDS], bit: usize) {
    let word = bit / 64;
    let mask = 1u64 << (bit % 64);
    bitmap[word] |= mask;
}

fn clear_bit(bitmap: &mut [u64; BITMAP_WORDS], bit: usize) {
    let word = bit / 64;
    let mask = 1u64 << (bit % 64);
    bitmap[word] &= !mask;
}

fn find_free_run(bitmap: &[u64; BITMAP_WORDS], nbits: usize, n: usize) -> Option<usize> {
    let mut run = 0usize;
    let mut run_start = 0usize;
    for bit in 0..nbits {
        if test_bit(bitmap, bit) {
            run = 0;
            continue;
        }
        if run == 0 {
            run_start = bit;
        }
        run += 1;
        if run == n {
            return Some(run_start);
        }
    }
    None
}

fn page_bit(mm: &MmState, addr: u64) -> Option<usize> {
    if mm.base == 0 || addr < mm.base || !addr.is_multiple_of(PAGE_SIZE) {
        return None;
    }
    let bit = ((addr - mm.base) / PAGE_SIZE) as usize;
    if bit < mm.nbits { Some(bit) } else { None }
}

#[cfg(debug_assertions)]
fn poison_pages(addr: u64, n: usize) {
    let Some(bytes) = (n as u64).checked_mul(PAGE_SIZE) else {
        return;
    };
    // SAFETY: pages are still marked used in the bitmap. Exclusive to this
    // hart under IrqCell (IRQs off). Range is page-aligned RAM in the pool.
    unsafe {
        core::ptr::write_bytes(addr as *mut u8, FREE_POISON, bytes as usize);
    }
}
