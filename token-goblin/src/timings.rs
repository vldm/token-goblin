//! Simple timing collector.
//!
//! Usage:
//! Wrap interesting part with `timed!` macro.
//! The result will be printed at the end of ROOT macro invocation, the root is macro
//! from whitelist: "munch" | "proxy" | "snif"
//!
//! Note: Printings is done only if `crate::PRINT_TIMINGS` is enabled.

use std::cell::RefCell;

thread_local! {
    /// Collector of timings.
    static TIMING: RefCell<Vec<(&'static str, std::time::Duration)>> = const { RefCell::new(Vec::new())};
}
fn take_timings() -> Vec<(&'static str, std::time::Duration)> {
    TIMING.with(|timings| std::mem::take(&mut *timings.borrow_mut()))
}
fn print_timings() {
    let timings = take_timings();
    for (name, duration) in timings {
        debug!("{}: {:?}", name, duration);
    }
}
pub(crate) fn save_timing(name: &'static str, duration: std::time::Duration) {
    TIMING.with(|timings| {
        timings.borrow_mut().push((name, duration));
    });
    // We measure only at end, if it's root macro - print all timings.
    let is_root = matches!(
        name,
        "munch"
            | "proxy"
            | "spit"
            | "derive_spit"
            | "derive_snif"
            | "derive_snif_attr"
            | "snif"
            | "vanish"
    );
    if is_root && crate::PRINT_TIMINGS {
        print_timings();
    }
}

macro_rules! timed {
    ($name: literal, $block: block) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let duration = start.elapsed();
        $crate::timings::save_timing($name, duration);
        result
    }};
}
