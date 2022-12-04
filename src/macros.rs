//! Logging, Debugging, Tracing, and Utility Macros
// probaly should just use the log crate instead of this but I'm lazy

macro_rules! log {
    ([$($tag_args:expr),+] $arg_literal:literal$(,$args:expr)*) => {
        println!(concat!("[", $($tag_args),+, "\x1b[0m] ", $arg_literal)$(,$args)*);
    };
}

/// overly complex debug and trace logging macro
#[cfg(any(feature = "debug", feature = "trace"))]
macro_rules! _log {
    ($($prefix:literal)?[$color:tt, $tag:literal, $local:literal] $($arg_literal:literal$(,$args:expr)*)?) => {
        eprintln!(concat!($crate::fg!(238), $($prefix,)?"[", $crate::fg!($color), $tag, fg!(238), " {:>w$}] ", $($arg_literal,)? "\x1b[0m"), concat!(module_path!(), "::", $local)$($(,$args)*)?, w = 30);
    };
}

// # Logging
#[macro_export]
macro_rules! error {
    ($($args:expr),+) => {
        if $crate::LogLevel::Error.enabled() { log!([fg!(Red), "ERROR"] $($args),+); }
    };
}

#[macro_export]
macro_rules! warn {
    ($($args:expr),+) => {
        if $crate::LogLevel::Warn.enabled() { log!([fg!(Yellow), "WARNING"] $($args),+); }
    };
}

#[macro_export]
macro_rules! info {
    ($($args:expr),+) => {
        if $crate::LogLevel::Info.enabled() { log!([fg!(Green), "INFO"] $($args),+); }
    };
}

// # Debugging
#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debug {
    //($($_:tt)*) => {};
    ($local:expr, $($args:expr),+) => {};
}
#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debug {
    ($local:expr, $($args:expr),+) => {
        _log!([blue,"DEBUG",$local] $($args),+);
    };
}

// # Tracing
#[cfg(not(feature = "trace"))]
#[macro_export]
macro_rules! trace {
    ($($_:tt)*) => {};
}
#[cfg(feature = "trace")]
#[macro_use]
mod tracing {
    macro_rules! _trace_end {
        ($local:expr) => {{
            let start = ::std::time::Instant::now();
            $crate::DropFn::new(move || {
                _log!([magenta, "TRACE", $local] "took {:?}", start.elapsed());
            })
        }};
    }
    #[macro_export]
    macro_rules! trace {
        ($local:expr) => {
            _log!([magenta, "TRACE", $local]);
            let __trace_end__ = _trace_end!($local);
        };
        ($id:ident = $local:expr) => {
            _log!([magenta, "TRACE", $local]);
            let $id = _trace_end!($local);
        };
    }
}

/// A simple and unsafe foot-gun macro that creates a new uninitialized variable.
/// Useful for unsafe write-before-read optimizations, avoiding memset and other unnecessary work.
///
/// # Example
/// ```
/// use crate::uninit;
/// fn main() {
///     let mut x: Vec<u32> = Vec::with_capacity(100);
///     x.resize(100, uninit!());
///     // x is now a Vec<u32> with 100 uninitialized elements
///
///     // which is the same as, and will *probably* get optimized to the same as:
///     let mut x: Vec<u32> = Vec::with_capacity(100);
///     unsafe { x.set_len(100) };
/// }
/// ```
/// # Safety
/// The variable must be initialized before it is read from.
#[macro_export]
macro_rules! uninit {
    () => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::uninit().assume_init()
        }
    };
}
