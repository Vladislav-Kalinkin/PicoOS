use crate::drivers::uart;

fn prefix(level: &str, scope: &str) {
    uart::write_str("[");
    uart::write_str(level);
    uart::write_str("][");
    uart::write_str(scope);
    uart::write_str("] ");
}

pub fn info(scope: &str, message: &str) {
    prefix("INFO", scope);
    uart::write_line(message);
}

pub fn ok(scope: &str, message: &str) {
    prefix("OK", scope);
    uart::write_line(message);
}

pub fn fail(scope: &str, message: &str) {
    prefix("FAIL", scope);
    uart::write_line(message);
}
