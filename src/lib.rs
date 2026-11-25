#![cfg_attr(not(test), no_std)]

mod checksum;
mod data;
mod error;
mod line;
mod rmc;

pub use checksum::verify_checksum;
pub use error::ParseError;
pub use line::{LineAssembler, MAX_LINE};
pub use rmc::RMC;
