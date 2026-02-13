//! Contains IO associated traits for internal Read/Write operations based on the
//! [`Transport`] trait.

mod net;

pub(crate) mod err;
pub(crate) mod read;
pub(crate) mod write;

pub use net::Transport;
