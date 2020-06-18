//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.

pub mod decode;
pub mod encode;
mod error;
mod parse;
mod token;

pub use decode::Node;
pub use encode::{Encode, Encoder};
pub use error::Error;
pub use parse::Parser;
pub use token::Token;

pub type Result<T> = std::result::Result<T, Error>;
