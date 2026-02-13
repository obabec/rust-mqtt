#![no_std]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(clippy::missing_safety_doc)]
#![deny(clippy::unnecessary_safety_doc)]
#![deny(clippy::unnecessary_safety_comment)]

#[cfg(test)]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

use embedded_io_async as eio;

#[cfg(all(feature = "bump", feature = "alloc"))]
compile_error!("You may not enable both `bump` and `alloc` features.");

#[cfg(all(feature = "log", feature = "defmt"))]
compile_error!("You may not enable both `log` and `defmt` features.");

#[cfg(all(test, not(any(feature = "bump", feature = "alloc"))))]
compile_error!("Enable either one of `bump` or `alloc` features for testing.");

mod bytes;
mod fmt;
mod packet;

pub mod buffer;
pub mod client;
pub mod config;
pub mod header;
pub mod io;
pub mod session;
pub mod types;

pub use bytes::Bytes;

#[cfg(test)]
mod test;

#[cfg(feature = "v3")]
mod v3;
#[cfg(feature = "v5")]
mod v5;
