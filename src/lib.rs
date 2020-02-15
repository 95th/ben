//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.

mod decode;
mod encode;
mod parse;

pub use decode::{Dict, List, Node};
pub use encode::Encoder;
pub use parse::Error;

pub type Result<T> = std::result::Result<T, Error>;
