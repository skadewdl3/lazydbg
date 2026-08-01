//! Typed client-side support for the Debug Adapter Protocol.

#![forbid(unsafe_code)]

pub mod client;
pub mod codec;
pub mod error;
#[allow(clippy::all)]
#[rustfmt::skip]
mod generated;
pub mod lldb;
pub mod protocol;
pub mod request;
pub mod requests;

pub use generated::*;
pub use request::DapRequest;
