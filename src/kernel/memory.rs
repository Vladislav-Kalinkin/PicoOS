use crate::drivers::uart;
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

static mut NEXT_FREE_PAGE: u64 = 0;
static mut MEMORY_END: u64 = 0;

pub fn init() {
    let start = align_up(free_memory_start(), PAGE_SIZE);
    let end = platform::RAM_START as u64 + platform::RAM_SIZE as u64;

    unsafe {
        NEXT_FREE_PAGE = start;
        MEMORY_END = end;
    }
}

pub fn allocate_page() -> Option<u64> {
    unsafe {
        if NEXT_FREE_PAGE == 0 || MEMORY_END == 0 {
            return None;
        }

        let page = NEXT_FREE_PAGE;
        let next = page + PAGE_SIZE;

        if next > MEMORY_END {
            return None;
        }

        NEXT_FREE_PAGE = next;

        Some(page)
    }
}

pub fn free_memory_current() -> u64 {
    unsafe { NEXT_FREE_PAGE }
}

pub fn memory_end() -> u64 {
    unsafe { MEMORY_END }
}

pub fn kernel_start() -> u64 {
    unsafe { &__kernel_start as *const u8 as u64 }
}

pub fn kernel_end() -> u64 {
    unsafe { &__kernel_end as *const u8 as u64 }
}

pub fn text_start() -> u64 {
    unsafe { &__text_start as *const u8 as u64 }
}

pub fn text_end() -> u64 {
    unsafe { &__text_end as *const u8 as u64 }
}

pub fn rodata_start() -> u64 {
    unsafe { &__rodata_start as *const u8 as u64 }
}

pub fn rodata_end() -> u64 {
    unsafe { &__rodata_end as *const u8 as u64 }
}

pub fn data_start() -> u64 {
    unsafe { &__data_start as *const u8 as u64 }
}

pub fn data_end() -> u64 {
    unsafe { &__data_end as *const u8 as u64 }
}

pub fn bss_start() -> u64 {
    unsafe { &__bss_start as *const u8 as u64 }
}

pub fn bss_end() -> u64 {
    unsafe { &__bss_end as *const u8 as u64 }
}

pub fn stack_top() -> u64 {
    unsafe { &__stack_top as *const u8 as u64 }
}

pub fn free_memory_start() -> u64 {
    unsafe { &__free_memory_start as *const u8 as u64 }
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
    uart::write_dec_u64(end - start);
    uart::write_line(" bytes");
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

#[allow(dead_code)]
pub fn kernel_text_start() -> u64 {
    unsafe { &__text_start as *const u8 as usize as u64 }
}

#[allow(dead_code)]
pub fn kernel_text_end() -> u64 {
    unsafe { &__text_end as *const u8 as usize as u64 }
}

#[allow(dead_code)]
pub fn is_inside_kernel_text(addr: u64) -> bool {
    addr >= kernel_text_start() && addr < kernel_text_end()
}
