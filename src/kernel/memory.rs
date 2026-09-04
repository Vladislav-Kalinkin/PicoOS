use crate::drivers::uart;
use crate::kernel::irq_cell::IrqCell;
use crate::platform;

pub const PAGE_SIZE: u64 = 4096;

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

struct MmState {
    next_free_page: u64,
    memory_end: u64,
}

static MM: IrqCell<MmState> = IrqCell::new(MmState {
    next_free_page: 0,
    memory_end: 0,
});

#[inline(always)]
fn symbol_addr(symbol: *const u8) -> u64 {
    symbol as u64
}

pub fn init() {
    let end = (platform::RAM_START as u64).checked_add(platform::RAM_SIZE as u64);
    let start = align_up(free_memory_start(), PAGE_SIZE);

    let (Some(start), Some(end)) = (start, end) else {
        MM.with(|mm| {
            mm.next_free_page = 0;
            mm.memory_end = 0;
        });
        return;
    };

    MM.with(|mm| {
        mm.next_free_page = if start <= end { start } else { 0 };
        mm.memory_end = end;
    });
}

pub fn allocate_page() -> Option<u64> {
    MM.with(|mm| {
        if mm.next_free_page == 0 || mm.memory_end == 0 {
            return None;
        }

        let page = mm.next_free_page;
        let next = page.checked_add(PAGE_SIZE)?;

        if next > mm.memory_end {
            return None;
        }

        mm.next_free_page = next;
        Some(page)
    })
}

pub fn free_memory_current() -> u64 {
    MM.with(|mm| mm.next_free_page)
}

pub fn memory_end() -> u64 {
    MM.with(|mm| mm.memory_end)
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

    allocate_and_print();
    allocate_and_print();
    allocate_and_print();

    uart::write_str("next free page: ");
    uart::write_hex_u64(free_memory_current());
    uart::write_line("");
}

fn allocate_and_print() {
    match allocate_page() {
        Some(page) => {
            uart::write_str("allocated page: ");
            uart::write_hex_u64(page);
            uart::write_line("");
        }
        None => {
            uart::write_line("allocated page: FAILED");
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
