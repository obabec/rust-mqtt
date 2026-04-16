#![allow(unused)]

#[clippy::format_args]
macro_rules! const_assert_ {
    ($($x:tt)*) => {
        {
            ::core::assert!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! assert_ {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt"))]
            ::core::assert!($($x)*);
            #[cfg(feature = "defmt")]
            ::defmt::assert!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! const_debug_assert_ {
    ($($x:tt)*) => {
        {
            ::core::debug_assert!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! debug_assert_ {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt"))]
            ::core::debug_assert!($($x)*);
            #[cfg(feature = "defmt")]
            ::defmt::debug_assert!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! debug_assert_eq_ {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt"))]
            ::core::debug_assert_eq!($($x)*);
            #[cfg(feature = "defmt")]
            ::defmt::debug_assert_eq!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! debug_assert_ne_ {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt"))]
            ::core::debug_assert_ne!($($x)*);
            #[cfg(feature = "defmt")]
            ::defmt::debug_assert_ne!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! unreachable_ {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt"))]
            ::core::unreachable!($($x)*);
            #[cfg(feature = "defmt")]
            ::defmt::unreachable!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! panic_ {
    ($($x:tt)*) => {
        {
            #[cfg(not(feature = "defmt"))]
            ::core::panic!($($x)*);
            #[cfg(feature = "defmt")]
            ::defmt::panic!($($x)*);
        }
    };
}

#[clippy::format_args]
macro_rules! verbose {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(all(feature = "log", feature = "log-verbose"))]
            ::log::trace!($s $(, $x)*);
            #[cfg(all(feature = "defmt", feature = "log-verbose"))]
            ::defmt::trace!($s $(, $x)*);
            #[cfg(not(any(
                all(feature = "log", feature = "log-verbose"),
                all(feature = "defmt", feature = "log-verbose"),
            )))]
            let _ = ($( & $x ),*);
        }
    };
}

#[clippy::format_args]
macro_rules! trace {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(all(feature = "log", feature = "log-level-trace"))]
            ::log::trace!($s $(, $x)*);
            #[cfg(all(feature = "defmt", feature = "log-level-trace"))]
            ::defmt::trace!($s $(, $x)*);
            #[cfg(not(any(
                all(feature = "log", feature = "log-level-trace"),
                all(feature = "defmt", feature = "log-level-trace"),
            )))]
            let _ = ($( & $x ),*);
        }
    };
}

#[clippy::format_args]
macro_rules! debug {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(all(feature = "log", feature = "log-level-debug"))]
            ::log::debug!($s $(, $x)*);
            #[cfg(all(feature = "defmt", feature = "log-level-debug"))]
            ::defmt::debug!($s $(, $x)*);
            #[cfg(not(any(
                all(feature = "log", feature = "log-level-debug"),
                all(feature = "defmt", feature = "log-level-debug"),
            )))]
            let _ = ($( & $x ),*);
        }
    };
}

#[clippy::format_args]
macro_rules! info {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(all(feature = "defmt", feature = "log-level-info"))]
            ::defmt::info!($s $(, $x)*);
            #[cfg(all(feature = "log", feature = "log-level-info"))]
            ::log::info!($s $(, $x)*);
            #[cfg(not(any(
                all(feature = "log", feature = "log-level-info"),
                all(feature = "defmt", feature = "log-level-info"),
            )))]
            let _ = ($( & $x ),*);
        }
    };
}

#[clippy::format_args]
macro_rules! warn_ {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(all(feature = "log", feature = "log-level-warn"))]
            ::log::warn!($s $(, $x)*);
            #[cfg(all(feature = "defmt", feature = "log-level-warn"))]
            ::defmt::warn!($s $(, $x)*);
            #[cfg(not(any(
                all(feature = "log", feature = "log-level-warn"),
                all(feature = "defmt", feature = "log-level-warn"),
            )))]
            let _ = ($( & $x ),*);
        }
    };
}

#[clippy::format_args]
macro_rules! error {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(all(feature = "log", feature = "log-level-error"))]
            ::log::error!($s $(, $x)*);
            #[cfg(all(feature = "defmt", feature = "log-level-error"))]
            ::defmt::error!($s $(, $x)*);
            #[cfg(not(any(
                all(feature = "log", feature = "log-level-error"),
                all(feature = "defmt", feature = "log-level-error"),
            )))]
            let _ = ($( & $x ),*);
        }
    };
}

pub(crate) use debug;
pub(crate) use error;
pub(crate) use info;
pub(crate) use trace;
pub(crate) use verbose;
pub(crate) use warn_ as warn;

pub(crate) use const_assert_ as const_assert;
pub(crate) use const_debug_assert_ as const_debug_assert;

pub(crate) use assert_ as assert;

pub(crate) use debug_assert_ as debug_assert;
pub(crate) use debug_assert_eq_ as debug_assert_eq;
pub(crate) use debug_assert_ne_ as debug_assert_ne;

pub(crate) use panic_ as panic;
pub(crate) use unreachable_ as unreachable;
