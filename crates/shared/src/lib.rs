#![feature(error_generic_member_access)]

pub mod error;
pub mod models;

pub use error::{Error, Result};
pub use models::*;
