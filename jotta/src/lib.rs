#![doc = include_str!("../README.md")]
#![warn(
    unreachable_pub,
    missing_debug_implementations,
    missing_docs,
    clippy::pedantic
)]

pub mod api;
pub mod auth;
mod client;
mod errors;
pub mod events;
pub mod files;
pub mod jfs;
pub mod path;
pub mod range;
pub(crate) mod serde;

pub(crate) type Result<T> = core::result::Result<T, errors::Error>;

pub use errors::Error;
pub use client::*;
