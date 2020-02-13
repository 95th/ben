//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.

mod node;
mod parse;

pub use node::Node;
pub use parse::Error;

pub type Result<T> = std::result::Result<T, Error>;
