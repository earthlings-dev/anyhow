//! Backtrace capture and re-export utilities.
//!
//! This module provides conditional backtrace support based on compiler features:
//!
//! - With `std_backtrace` (MSRV 1.93): Uses `std::backtrace::Backtrace`
//! - Without `std_backtrace` (no-std): Provides an empty stub type

/// Re-export of `std::backtrace` types when available.
#[cfg(std_backtrace)]
pub(crate) use std::backtrace::{Backtrace, BacktraceStatus};

/// Empty stub type for no-std builds where backtrace is unavailable.
#[cfg(not(std_backtrace))]
pub(crate) enum Backtrace {}

/// Expands to the concrete backtrace type for use in public API signatures.
#[cfg(std_backtrace)]
macro_rules! impl_backtrace {
    () => {
        std::backtrace::Backtrace
    };
}

/// Captures a new backtrace at the current call site.
///
/// Returns `Some(Backtrace)` when std backtrace is available, `None` otherwise.
#[cfg(std_backtrace)]
macro_rules! backtrace {
    () => {
        Some(crate::backtrace::Backtrace::capture())
    };
}

/// No-op backtrace capture for no-std builds.
#[cfg(not(std_backtrace))]
macro_rules! backtrace {
    () => {
        None
    };
}

/// Captures a backtrace only if the error doesn't already provide one.
///
/// With `error_generic_member_access`, checks if the error provides a backtrace
/// via the `provide` API before capturing a new one.
#[cfg(error_generic_member_access)]
macro_rules! backtrace_if_absent {
    ($err:expr) => {
        match $crate::nightly::request_ref_backtrace($err as &dyn core::error::Error) {
            Some(_) => None,
            None => backtrace!(),
        }
    };
}

/// Always captures a backtrace when generic member access is unavailable.
#[cfg(all(feature = "std", not(error_generic_member_access)))]
macro_rules! backtrace_if_absent {
    ($err:expr) => {
        backtrace!()
    };
}

/// No backtrace capture in no-std builds without generic member access.
#[cfg(all(not(feature = "std"), not(error_generic_member_access)))]
macro_rules! backtrace_if_absent {
    ($err:expr) => {
        None
    };
}

/// Compile-time assertion that `Backtrace` implements `Send + Sync`.
#[cfg(std_backtrace)]
fn _assert_send_sync() {
    fn assert<T: Send + Sync>() {}
    assert::<Backtrace>();
}
