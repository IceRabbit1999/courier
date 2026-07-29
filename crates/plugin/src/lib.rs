#![feature(error_generic_member_access)]

pub mod channel;
pub mod client;
pub mod error;

pub use client::Client;
pub use error::*;
