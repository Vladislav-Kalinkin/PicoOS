use core::cell::UnsafeCell;

use crate::drivers::uart;
use crate::kernel::hart_local::HartLocal;
use crate::platform::{self, HART_SLOTS};

pub const PAGE_SIZE: u64 = 4096;

/// Static bitmap capacity: 512 × 64 bits = 32768 pages = 128 MiB from `RAM_START`.
const BITMAP_WORDS: usize = 512;
const BITMAP_BITS: usize = BITMAP_WORDS * 64;

const WORDS_PER_HART: usize = BITMAP_WORDS / HART_SLOTS;
const BITS_PER_HART: usize = WORDS_PER_HART * 64;

const _: () = assert!(BITMAP_WORDS.is_multiple_of(HART_SLOTS));
const _: () = assert!(HART_SLOTS == 8);

#[cfg(debug_assertions)]
const FREE_POISON: u8 = 0xA5;

unsafe extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;

    static __text_start: u8;
    static __text_end: u8;

    static __user_text_start: u8;
    static __user_text_end: u8;

    static __rodata_start: u8;
    static __rodata_end: u8;

    static __data_start: u8;
    static __data_end: u8;

    static __bss_start: u8;
    static __bss_end: u8;

    static __stack_top: u8;
    static __free_memory_start: u8;
}

/// Shared physical bitmap storage in `.bss`.
///
/// Disjoint ranges of 64-bit words are assigned strictly to individual harts.
/// Harts never touch or share words across boundary ranges.
struct SharedBitmap {
    words: [UnsafeCell<u64>; BITMAP_WORDS],
}

// SAFETY: access to each word in `words` is strictly partitioned by `word_offset`
// per hart slot. No two harts mutate or read the same 64-bit word simultaneously.
unsafe impl Sync for SharedBitmap {}

static BITMAP: SharedBitmap = SharedBitmap {
    words: [const { UnsafeCell::new(0) }; BITMAP_WORDS],
};

#[derive(Clone, Copy)]
struct MmHart {
    base_page_addr: u64,
    word_offset: usize,
    valid_bits: usize,
    used: u64,
    high_water: u64,
}

impl MmHart {
    const fn empty() -> Self {
        Self {
            base_page_addr: 0,
            word_offset: 0,
            valid_bits: 0,
            used: 0,
            high_water: 0,
        }
    }
}

static MM_HARTS: HartLocal<MmHart> = HartLocal::new(MmHart::empty());

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

/// Page counts for the managed pool across the whole system or local hart.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MmStats {
    pub free: u64,
    pub used: u64,
    pub high_water: u64,
}

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

/// Partition bitmap into `HART_SLOTS` word-aligned ranges. Hart 0 only.
pub fn init() {
    let ram_end = (platform::RAM_START as u64).checked_add(platform::RAM_SIZE as u64);
    let start = align_up(free_memory_start(), PAGE_SIZE);

    let (Some(start), Some(ram_end)) = (start, ram_end) else {
        return;
    };
    if start >= ram_end {
        return;
    }

    let span = ram_end - start;
    let mut total_valid_bits = (span / PAGE_SIZE) as usize;
    if total_valid_bits > BITMAP_BITS {
        total_valid_bits = BITMAP_BITS;
    }

    // Initialize all hart partitions
    for slot in 0..HART_SLOTS {
        let word_offset = slot * WORDS_PER_HART;
        let hart_bit_start = slot * BITS_PER_HART;
        let hart_page_base = start + (hart_bit_start as u64) * PAGE_SIZE;

        let valid_bits = if total_valid_bits > hart_bit_start {
            core::cmp::min(total_valid_bits - hart_bit_start, BITS_PER_HART)
        } else {
            0
        };

        // Mark bits outside valid RAM as used/allocated (stuck invalid)
        for bit in 0..BITS_PER_HART {
            let global_word = word_offset + (bit / 64);
            let bit_mask = 1u64 << (bit % 64);

            let word_ptr = BITMAP.words[global_word].get();
            // SAFETY: Single-hart boot path prior to bringing up secondary harts.
            unsafe {
                if bit >= valid_bits {
                    *word_ptr |= bit_mask;
                } else {
                    *word_ptr &= !bit_mask;
                }
            }
        }

        // Write configuration into each hart's slot directly
        hart_local_set_slot(
            slot,
            MmHart {
                base_page_addr: hart_page_base,
                word_offset,
                valid_bits,
                used: 0,
                high_water: 0,
            },
        );
    }
}

/// Allocate `n` consecutive pages from the executing hart's local range.
///
/// Forbidden on the IRQ/trap prologue path. Runs with interrupts disabled.
pub fn alloc_pages(n: usize) -> Option<PhysPage> {
    if n == 0 {
        return None;
    }

    MM_HARTS.with(|mm| {
        if mm.base_page_addr == 0 || mm.valid_bits == 0 || n > mm.valid_bits {
            return None;
        }

        let start_bit = find_free_run(mm.word_offset, mm.valid_bits, n)?;
        for bit in start_bit..start_bit + n {
            set_partition_bit(mm.word_offset, bit);
        }

        mm.used += n as u64;
        if mm.used > mm.high_water {
            mm.high_water = mm.used;
        }

        Some(PhysPage {
            addr: mm.base_page_addr + (start_bit as u64) * PAGE_SIZE,
        })
    })
}

/// Free `n` consecutive pages previously returned by [`alloc_pages`].
/// Must be freed by the owning hart within its partition.
pub fn free_pages(base: PhysPage, n: usize) {
    if n == 0 {
        return;
    }

    MM_HARTS.with(|mm| {
        let Some(start_bit) = page_to_bit(mm, base.addr) else {
            return;
        };
        let Some(end_bit) = start_bit.checked_add(n) else {
            return;
        };
        if end_bit > mm.valid_bits {
            return;
        }

        for bit in start_bit..end_bit {
            if !test_partition_bit(mm.word_offset, bit) {
                return;
            }
        }

        #[cfg(debug_assertions)]
        poison_pages(base.addr, n);

        for bit in start_bit..end_bit {
            clear_partition_bit(mm.word_offset, bit);
        }

        mm.used = mm.used.saturating_sub(n as u64);
    });
}

pub fn stats() -> MmStats {
    let mut total_free = 0u64;
    let mut total_used = 0u64;
    let mut max_high_water = 0u64;

    for slot in 0..HART_SLOTS {
        let info = hart_local_get_slot(slot);
        total_free += (info.valid_bits as u64).saturating_sub(info.used);
        total_used += info.used;
        if info.high_water > max_high_water {
            max_high_water = info.high_water;
        }
    }

    MmStats {
        free: total_free,
        used: total_used,
        high_water: max_high_water,
    }
}

pub fn allocate_page() -> Option<u64> {
    alloc_pages(1).map(PhysPage::addr)
}

pub fn free_memory_current() -> u64 {
    MM_HARTS.with(|mm| {
        if mm.base_page_addr == 0 || mm.valid_bits == 0 {
            return 0;
        }
        match find_free_run(mm.word_offset, mm.valid_bits, 1) {
            Some(bit) => mm.base_page_addr + (bit as u64) * PAGE_SIZE,
            None => 0,
        }
    })
}

pub fn memory_end() -> u64 {
    MM_HARTS.with(|mm| {
        if mm.base_page_addr == 0 {
            0
        } else {
            mm.base_page_addr + (mm.valid_bits as u64) * PAGE_SIZE
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

pub fn user_text_start() -> u64 {
    symbol_addr(core::ptr::addr_of!(__user_text_start))
}

pub fn user_text_end() -> u64 {
    symbol_addr(core::ptr::addr_of!(__user_text_end))
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

pub fn is_inside_user_text(addr: u64) -> bool {
    addr >= user_text_start() && addr < user_text_end()
}

pub fn print_memory_layout() {
    uart::write_line("");
    uart::write_line("memory layout:");

    print_range("kernel", kernel_start(), kernel_end());
    print_range("text", text_start(), text_end());
    print_range("usertext", user_text_start(), user_text_end());
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

fn test_partition_bit(word_offset: usize, bit: usize) -> bool {
    let global_word = word_offset + (bit / 64);
    let mask = 1u64 << (bit % 64);
    // SAFETY: caller passes partitioned offset; accesses only owning hart's words.
    unsafe { (*BITMAP.words[global_word].get()) & mask != 0 }
}

fn set_partition_bit(word_offset: usize, bit: usize) {
    let global_word = word_offset + (bit / 64);
    let mask = 1u64 << (bit % 64);
    // SAFETY: caller passes partitioned offset; accesses only owning hart's words.
    unsafe {
        *BITMAP.words[global_word].get() |= mask;
    }
}

fn clear_partition_bit(word_offset: usize, bit: usize) {
    let global_word = word_offset + (bit / 64);
    let mask = 1u64 << (bit % 64);
    // SAFETY: caller passes partitioned offset; accesses only owning hart's words.
    unsafe {
        *BITMAP.words[global_word].get() &= !mask;
    }
}

fn find_free_run(word_offset: usize, valid_bits: usize, n: usize) -> Option<usize> {
    let mut run = 0usize;
    let mut run_start = 0usize;

    for bit in 0..valid_bits {
        if test_partition_bit(word_offset, bit) {
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

fn page_to_bit(mm: &MmHart, addr: u64) -> Option<usize> {
    if mm.base_page_addr == 0 || addr < mm.base_page_addr || !addr.is_multiple_of(PAGE_SIZE) {
        return None;
    }
    let bit = ((addr - mm.base_page_addr) / PAGE_SIZE) as usize;
    if bit < mm.valid_bits { Some(bit) } else { None }
}

#[cfg(debug_assertions)]
fn poison_pages(addr: u64, n: usize) {
    let Some(bytes) = (n as u64).checked_mul(PAGE_SIZE) else {
        return;
    };
    // SAFETY: Pages remain marked as used during poisoning; exclusive to this
    // hart under HartLocal (IRQs disabled). Target range is page-aligned RAM.
    unsafe {
        core::ptr::write_bytes(addr as *mut u8, FREE_POISON, bytes as usize);
    }
}

/// Helper to set metadata across slots during boot partition init (Hart 0 only).
fn hart_local_set_slot(slot: usize, val: MmHart) {
    // SAFETY: Only invoked on Hart 0 during early boot prior to starting secondaries.
    unsafe {
        *MM_HARTS.slot_ptr(slot) = val;
    }
}

/// Helper to read stats across slots.
fn hart_local_get_slot(slot: usize) -> MmHart {
    // SAFETY: Slot metadata is initialized during boot; stats only performs reads.
    unsafe { *MM_HARTS.slot_ptr(slot) }
}
