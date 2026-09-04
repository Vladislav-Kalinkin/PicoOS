use crate::drivers::mmio;
use crate::platform;

pub fn putc(byte: u8) {
    unsafe {
        mmio::write32(platform::UART0_BASE, byte as u32);
    }
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            putc(b'\r');
        }

        putc(byte);
    }
}

pub fn write_line(s: &str) {
    write_str(s);
    write_str("\n");
}

pub fn write_hex_u64(value: u64) {
    write_str("0x");

    let mut shift = 60;

    loop {
        let digit = ((value >> shift) & 0xF) as u8;

        let ch = if digit < 10 {
            b'0' + digit
        } else {
            b'A' + (digit - 10)
        };

        putc(ch);

        if shift == 0 {
            break;
        }

        shift -= 4;
    }
}

pub fn write_dec_u64(mut value: u64) {
    if value == 0 {
        putc(b'0');
        return;
    }

    let mut buffer = [0u8; 20];
    let mut index = buffer.len();

    while value > 0 {
        index -= 1;
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    while index < buffer.len() {
        putc(buffer[index]);
        index += 1;
    }
}
