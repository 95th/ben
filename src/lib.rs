//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.

#![no_std]

use core::ops::Range;

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub size: usize,
}

impl Token {
    pub fn new(kind: TokenKind, start: Option<usize>, end: Option<usize>) -> Self {
        Self::with_size(kind, start, end, 0)
    }

    pub fn with_size(
        kind: TokenKind,
        start: Option<usize>,
        end: Option<usize>,
        size: usize,
    ) -> Self {
        Self {
            kind,
            start,
            end,
            size,
        }
    }

    pub fn as_range(&self) -> Option<Range<usize>> {
        self.start.and_then(|start| self.end.map(|end| start..end))
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    None,
    Dict,
    List,
    ByteStr,
    Int,
    End,
}

impl Default for TokenKind {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Error {
    /// The string is not a full JSON packet, more bytes expected
    Part,
    /// Invalid character inside JSON string
    Invalid,
    /// Not enough tokens were provided
    NoMemory,
    /// Overflow (numeric or otherwise)
    Overflow,
    /// String parsing
    ParseStr,
}

pub struct BenDecoder {
    pos: usize,
    tok_next: usize,
    tok_super: Option<usize>,
}

impl Default for BenDecoder {
    fn default() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: None,
        }
    }
}

impl BenDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    ///
    /// Run Bencode parser. It parses a bencoded data string into an array of tokens, each
    /// describing a single Bencode object.
    ///
    /// Parse bencoded string and fill tokens.
    ///
    /// Returns number of tokens parsed.
    pub fn parse(&mut self, buf: &[u8], tokens: &mut [Token]) -> Result<usize, Error> {
        let mut count = self.tok_next;
        while self.pos < buf.len() {
            let c = buf[self.pos];
            match c {
                b'i' => {
                    count += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1;
                    }
                    self.pos += 1;
                    let start = self.pos;
                    self.parse_int(buf, b'e')?;
                    match self.alloc_token(tokens) {
                        Some(i) => {
                            tokens[i] = Token::new(TokenKind::Int, Some(start), Some(self.pos));
                        }
                        None => {
                            self.pos = start;
                            return Err(Error::Invalid);
                        }
                    }
                }
                b'l' => {
                    count += 1;
                    self.pos += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1;
                    }
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    tokens[i] = Token::new(TokenKind::List, Some(self.pos), None);
                    self.tok_super = Some(self.tok_next - 1);
                }
                b'd' => {
                    count += 1;
                    self.pos += 1;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1;
                    }
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    tokens[i] = Token::new(TokenKind::Dict, Some(self.pos), None);
                    self.tok_super = Some(self.tok_next - 1);
                }
                b'0'..=b'9' => {
                    count += 1;
                    self.parse_string(buf, tokens)?;
                    if let Some(i) = self.tok_super {
                        tokens[i].size += 1
                    }
                }
                b'e' => {
                    if let Some(i) = self.tok_super {
                        tokens[i].end = Some(self.pos);
                    }
                    break;
                }
                _ => {
                    // Unexpected char
                    return Err(Error::Invalid);
                }
            }
            self.pos += 1;
        }
        for i in (0..self.tok_next).rev() {
            // Unmatched opened object
            if tokens[i].start.is_some() && tokens[i].end.is_none() {
                return Err(Error::Part);
            }
        }
        Ok(count)
    }

    /// Fills next available token with bencode int.
    fn parse_int(&mut self, buf: &[u8], stop_char: u8) -> Result<i64, Error> {
        if self.pos >= buf.len() {
            return Err(Error::Invalid);
        }

        let mut negative = false;
        let mut val = 0;

        let start = self.pos;

        if buf[self.pos] == b'-' {
            self.pos += 1;
            negative = true;
            if self.pos == buf.len() {
                self.pos = start;
                return Err(Error::Invalid);
            }
        }

        while self.pos < buf.len() {
            match buf[self.pos] {
                c @ b'0'..=b'9' => {
                    if val > i64::max_value() / 10 {
                        self.pos = start;
                        return Err(Error::Overflow);
                    }
                    val *= 10;
                    let digit = (c - b'0') as i64;
                    if val > i64::max_value() - digit {
                        self.pos = start;
                        return Err(Error::Overflow);
                    }
                    val += digit;
                    self.pos += 1
                }
                c if c == stop_char => break,
                _ => {
                    self.pos = start;
                    return Err(Error::Invalid);
                }
            }
        }

        if negative {
            val *= -1;
        }
        Ok(val)
    }

    /// Fills next token with bencode string.
    fn parse_string(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<(), Error> {
        let start = self.pos;

        let len = self.parse_int(js, b':')?;
        self.pos += 1;

        if len <= 0 {
            self.pos = start;
            return Err(Error::ParseStr);
        }

        let len = len as usize;
        if self.pos + len > js.len() {
            self.pos = start;
            return Err(Error::ParseStr);
        }

        match self.alloc_token(tokens) {
            Some(i) => {
                tokens[i] = Token::new(TokenKind::ByteStr, Some(self.pos - len), Some(self.pos));
                Ok(())
            }
            None => {
                self.pos = start;
                Err(Error::NoMemory)
            }
        }
    }

    /// Allocates a fresh unused token from the token pool.
    fn alloc_token(&mut self, tokens: &mut [Token]) -> Option<usize> {
        if self.tok_next as usize >= tokens.len() {
            return None;
        }
        let idx = self.tok_next as usize;
        self.tok_next += 1;
        let tok = &mut tokens[idx];
        tok.end = None;
        tok.start = tok.end;
        tok.size = 0;
        Some(idx)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     macro_rules! parse {
//         ($buf: expr, $len: expr) => {{
//             let mut v = [Token::default(); $len];
//             let mut parser = JsonParser::new();
//             parser.parse($buf, &mut v).map(|parsed| {
//                 assert_eq!($len, parsed as usize);
//                 v
//             })
//         }};
//     }

//     #[test]
//     fn parse_int() {
//         let s = b"1234";
//         let tokens = parse!(s, 1).unwrap();
//         assert_eq!(
//             &[Token::new(TokenKind::Primitive, Some(0), Some(4))],
//             &tokens
//         );
//     }

//     #[test]
//     fn parse_int_negative() {
//         let s = b"-1234";
//         let tokens = parse!(s, 1).unwrap();
//         assert_eq!(
//             &[Token::new(TokenKind::Primitive, Some(0), Some(5))],
//             &tokens
//         );
//     }

//     #[test]
//     fn parse_int_invalid() {
//         let s = b"abc1234";
//         let err = parse!(s, 1).unwrap_err();
//         assert_eq!(Error::Invalid, err);
//     }

//     #[test]
//     fn parse_string() {
//         let s = br#""abcd""#;
//         let tokens = parse!(s, 1).unwrap();
//         assert_eq!(&[Token::new(TokenKind::Str, Some(1), Some(5))], &tokens);
//     }

//     #[test]
//     fn parse_object() {
//         let s = br#"{"a": "b", "c": 100}"#;
//         let tokens = parse!(s, 5).unwrap();
//         assert_eq!(
//             &[
//                 Token::with_size(TokenKind::Object, Some(0), Some(20), 2),
//                 Token::with_size(TokenKind::Str, Some(2), Some(3), 1),
//                 Token::with_size(TokenKind::Str, Some(7), Some(8), 0),
//                 Token::with_size(TokenKind::Str, Some(12), Some(13), 1),
//                 Token::with_size(TokenKind::Primitive, Some(16), Some(19), 0)
//             ],
//             &tokens
//         );
//     }

//     #[test]
//     fn parse_array() {
//         let s = br#"["a", "b", "c", 100]"#;
//         let tokens = parse!(s, 5).unwrap();
//         assert_eq!(
//             &[
//                 Token::with_size(TokenKind::Array, Some(0), Some(20), 4),
//                 Token::with_size(TokenKind::Str, Some(2), Some(3), 0),
//                 Token::with_size(TokenKind::Str, Some(7), Some(8), 0),
//                 Token::with_size(TokenKind::Str, Some(12), Some(13), 0),
//                 Token::with_size(TokenKind::Primitive, Some(16), Some(19), 0)
//             ],
//             &tokens
//         );
//     }

//     #[test]
//     fn parse_array_oom() {
//         let s = br#"["a", "b", "c", 100]"#;
//         let err = parse!(s, 4).unwrap_err();
//         assert_eq!(Error::NoMemory, err);
//     }
// }
