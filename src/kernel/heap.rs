use crate::drivers::uart;
use crate::kernel::memory;

const HEAP_PAGES: usize = 4;

static mut HEAP_START: u64 = 0;
static mut HEAP_END: u64 = 0;
static mut HEAP_NEXT: u64 = 0;

fn reset_heap_state() {
    unsafe {
        HEAP_START = 0;
        HEAP_END = 0;
        HEAP_NEXT = 0;
    }
}

pub fn init() -> bool {
    reset_heap_state();

    let first_page = memory::allocate_page();

    let Some(start) = first_page else {
        return false;
    };

    let mut last = start;

    for _ in 1..HEAP_PAGES {
        let Some(page) = memory::allocate_page() else {
            reset_heap_state();
            return false;
        };

        last = page;
    }

    let Some(end) = last.checked_add(memory::PAGE_SIZE) else {
        reset_heap_state();
        return false;
    };

    unsafe {
        HEAP_START = start;
        HEAP_END = end;
        HEAP_NEXT = start;
    }

    true
}

pub fn alloc(size: u64, align: u64) -> Option<u64> {
    if size == 0 || align == 0 || !align.is_power_of_two() {
        return None;
    }

    unsafe {
        if HEAP_START == 0 || HEAP_END == 0 || HEAP_NEXT == 0 {
            return None;
        }

        let start = align_up(HEAP_NEXT, align)?;
        let end = start.checked_add(size)?;

        if end > HEAP_END {
            return None;
        }

        HEAP_NEXT = end;

        Some(start)
    }
}

pub fn heap_start() -> u64 {
    unsafe { HEAP_START }
}

pub fn heap_end() -> u64 {
    unsafe { HEAP_END }
}

pub fn heap_next() -> u64 {
    unsafe { HEAP_NEXT }
}

pub fn heap_size() -> u64 {
    unsafe { HEAP_END.saturating_sub(HEAP_START) }
}

pub fn test_heap() {
    uart::write_line("");
    uart::write_line("heap:");

    if !init() {
        uart::write_line("heap init: FAILED");
        return;
    }

    uart::write_str("heap start: ");
    uart::write_hex_u64(heap_start());
    uart::write_line("");

    uart::write_str("heap end: ");
    uart::write_hex_u64(heap_end());
    uart::write_line("");

    uart::write_str("heap size: ");
    uart::write_dec_u64(heap_size());
    uart::write_line(" bytes");

    alloc_and_print(64, 8);
    alloc_and_print(128, 16);
    alloc_and_print(4096, 4096);
    alloc_and_print(8192, 4096);

    uart::write_str("next heap pointer: ");
    uart::write_hex_u64(heap_next());
    uart::write_line("");
}

fn alloc_and_print(size: u64, align: u64) {
    uart::write_str("alloc ");
    uart::write_dec_u64(size);
    uart::write_str(" bytes align ");
    uart::write_dec_u64(align);
    uart::write_str(": ");

    match alloc(size, align) {
        Some(addr) => {
            uart::write_hex_u64(addr);
            uart::write_line("");
        }
        None => {
            uart::write_line("FAILED");
        }
    }
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }

    Some(value.checked_add(align - 1)? & !(align - 1))
}
