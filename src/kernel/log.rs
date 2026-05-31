use crate::drivers::uart;

fn scope_enabled(scope: &str) -> bool {
    // Keep current behavior by default. Scope filtering activates only with kernel_log_scoped.
    if !cfg!(feature = "kernel_log_scoped") {
        return true;
    }

    if scope == "scheduler" {
        return cfg!(feature = "scheduler_verbose_dispatch_trace");
    }
    if scope == "resume" {
        return cfg!(feature = "verbose_resume_debug");
    }
    if scope == "trap" {
        return cfg!(feature = "log_trap");
    }
    if scope == "timer" {
        return cfg!(feature = "log_timer");
    }
    if scope == "fault" {
        return cfg!(feature = "log_fault");
    }
    if scope == "sleep" {
        return cfg!(feature = "log_sleep");
    }
    true
}

fn prefix(level: &str, scope: &str) {
    uart::write_str("[");
    uart::write_str(level);
    uart::write_str("][");
    uart::write_str(scope);
    uart::write_str("] ");
}

#[allow(dead_code)]
pub fn info(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("INFO", scope);
    uart::write_line(message);
}

#[allow(dead_code)]
pub fn ok(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("OK", scope);
    uart::write_line(message);
}

#[allow(dead_code)]
pub fn fail(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("FAIL", scope);
    uart::write_line(message);
}

#[cfg(any(feature = "scheduler_verbose_dispatch_trace", feature = "verbose_resume_debug"))]
#[allow(dead_code)]
pub fn trace(scope: &str, message: &str) {
    if !scope_enabled(scope) {
        return;
    }
    prefix("TRACE", scope);
    uart::write_line(message);
}

#[cfg(not(any(feature = "scheduler_verbose_dispatch_trace", feature = "verbose_resume_debug")))]
#[allow(dead_code)]
pub fn trace(_scope: &str, _message: &str) {}
