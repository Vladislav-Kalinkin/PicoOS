use crate::drivers::mmio;
use crate::platform;

#[allow(dead_code)]
pub fn init() {}

#[inline(always)]
pub fn putc(byte: u8) {
    mmio::write32(platform::UART0_BASE, byte as u32);
}

pub fn write_str(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            putc(b'\r');
        }

        putc(byte);
    }
}

pub fn write_line(text: &str) {
    write_str(text);
    write_str("\n");
}

pub fn write_hex_u64(value: u64) {
    write_str("0x");

    for i in (0..16).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;

        let ch = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'A' + (nibble - 10),
        };

        putc(ch);
    }
}

pub fn write_dec_u64(mut value: u64) {
    if value == 0 {
        putc(b'0');
        return;
    }

    let mut buffer = [0u8; 20];
    let mut index = 0;

    while value > 0 {
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
        index += 1;
    }

    while index > 0 {
        index -= 1;
        putc(buffer[index]);
    }
}
