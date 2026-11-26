#![cfg_attr(not(test), no_std)]

mod checksum;
mod data;
mod date;
mod error;
mod gga;
mod gsa;
mod line;
mod parse;
mod rmc;
mod sentence;

pub use checksum::verify_checksum;
pub use data::NmeaData;
pub use error::ParseError;
pub use line::{LineAssembler, MAX_LINE};
pub use rmc::build_rmc_struct;
