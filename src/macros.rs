#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debug {
    ($l:literal$(,$a:expr)*) => {
        eprintln!(concat!("[DEBUG {:>w$}] \t",$l), module_path!()$(,$a)*, w = 16);
    };
}
#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debug {
    ($($_:expr),*) => {};
}

#[macro_export]
macro_rules! error {
    ($l:literal$(,$a:expr)*) => {
        eprintln!(concat!("[\x1b[38;5;196mERROR\x1b[0m] ", $l)$(,$a)*);
    };
}

#[macro_export]
macro_rules! warning {
    ($l:literal$(,$a:expr)*) => {
        eprintln!(concat!("[\x1b[38;5;208mWARNING\x1b[0m] ", $l)$(,$a)*);
    };
}

#[macro_export]
macro_rules! info {
    ($l:literal$(,$a:expr)*) => {
        eprintln!(concat!("[\x1b[38;5;27mINFO\x1b[0m] ", $l)$(,$a)*);
    };
}

/// A simple and unsafe foot-gun macro that creates a new uninitialized variable.
/// Useful for unsafe write-before-read optimizations, avoiding memset and other unnecessary work.
///
/// You still get a warning about an invalid value, so it's not a silent foot-gun at least.
/// # Example
/// ```
/// use uninit::uninit;
/// fn main() {
///     let mut x: Vec<u32> = Vec::with_capacity(100);
///     x.resize(100, uninit!());
///     // x is now a Vec<u32> with 100 uninitialized elements
///
///     // which is the same as, and will probably get optimized to the same as:
///     let mut x: Vec<u32> = Vec::with_capacity(100);
///     unsafe { x.set_len(100) };
/// }
/// ```
/// # Safety
/// The variable must be initialized before it is read from.
#[macro_export]
macro_rules! uninit {
    () => {
        unsafe { ::core::mem::MaybeUninit::uninit().assume_init() }
    };
}

#[cfg(feature = "profiler")]
mod profiler {
    use core::fmt::Display;
    use std::{
        any::Any,
        time::{Duration, Instant},
    };
    /// An extremely simple timer that prints the time elapsed when dropped to stderr with the given label
    pub struct DropTimer<L: Display> {
        pub label: L,
        pub start: Instant,
    }
    impl<L: Display> DropTimer<L> {
        #[inline]
        pub fn new(label: L) -> Self { Self { label, start: Instant::now() } }
    }
    impl<L: Display> Drop for DropTimer<L> {
        #[inline]
        fn drop(&mut self) {
            eprintln!("{} took {:?}", self.label, self.start.elapsed());
        }
    }
    pub struct CustomDropTimer<L: Display + Clone + Any> {
        label: L,
        start: Instant,
        _drop: Box<dyn Fn(&Self)>,
    }
    impl<L: Display + Clone + Any> CustomDropTimer<L> {
        #[inline]
        pub fn new(label: L, custom_drop: Box<dyn Fn(&Self)>) -> Self {
            Self { label, start: Instant::now(), _drop: custom_drop }
        }
        #[inline]
        pub fn downcast_label<T: Any>(&self) -> Result<Box<T>, Box<dyn Any>> {
            let b: Box<dyn Any> = Box::new(self.label.clone());
            b.downcast()
        }
        #[inline]
        pub fn label(&self) -> &L { &self.label }
        #[inline]
        pub fn start(&self) -> &Instant { &self.start }
        #[inline]
        pub fn elapsed(&self) -> Duration { self.start.elapsed() }
        #[inline]
        pub fn default_drop(&self) {
            eprintln!("{} took {:?}", self.label(), self.elapsed());
        }
    }
    impl<L: Display + Clone + Any> Drop for CustomDropTimer<L> {
        #[inline]
        fn drop(&mut self) { (self._drop)(self); }
    }
}
#[cfg(feature = "profiler")]
pub use profiler::*;
/// An extremely simple timer that prints the time elapsed when dropped to stderr with the given label
/// and is a no-op if the profiler feature is not enabled
///
/// # Usage
/// ```
/// { timer!(); } // prints "example.rs:1:1 unnamed timer took 0ns"
/// { timer!("label"); } // prints "label took 0ns"
/// { timer!("label", |timer| eprintln!("custom drop after {:?}", timer.elapsed()) ); } // prints "custom drop after 0ns"
/// timer!(custom_variable_name_1, "label1"); // prints "label1 took 0ns"
/// timer!(custom_variable_name_2, "label2", |timer| { eprintln!("custom message with {}", timer.label()) }); // prints "custom message with label2"
/// ```
/// # Example
/// ```
/// use drop_timer::timer;
/// fn main() {
///   timer!("label");
///   // expands to: let __drop_timer__ = drop_timer::DropTimer::new("label");
///
///   // stuff that takes a second ...
/// }
/// // prints "label took 1.0001s" to stderr
/// ```
///
#[cfg(feature = "profiler")]
#[macro_export]
macro_rules! timer {
    // for when things start breaking (tracking down unsafe code):
    // #[cfg(feature = "profiler")]
    // eprintln!("timing {}...", $label);
    () => {
        let __drop_timer__ = crate::DropTimer::new(concat!(file!(), ":", line!(), ":", column!(), " unnamed timer"));
    };
    ($label:expr) => {
        let __drop_timer__ = crate::DropTimer::new($label);
    };
    ($id:ident, $label:expr) => {
        let $id = crate::DropTimer::new($label);
    };
    ($label:expr, $drop_fn:expr) => {
        let __drop_timer__ = crate::CustomDropTimer::new($label, Box::new($drop_fn));
    };
    ($id:ident, $label:expr, $drop_fn:expr) => {
        let $id = crate::CustomDropTimer::new($label, Box::new($drop_fn));
    };
}

#[cfg(not(feature = "profiler"))]
#[macro_export]
macro_rules! timer {
    ($($_:expr),*) => {};
}
