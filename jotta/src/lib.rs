#![doc = include_str!("../README.md")]
#![warn(
    unreachable_pub,
    missing_debug_implementations,
    missing_docs,
    clippy::pedantic
)]

mod bucket;
pub mod errors;
mod object;

pub use bucket::*;
pub use jotta_fs::{auth, Fs};
pub use object::*;

pub(crate) type Result<T> = core::result::Result<T, errors::Error>;
