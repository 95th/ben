//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.

#![no_std]

use core::ops::Range;

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: isize,
    pub end: isize,
    pub size: usize,
}

impl Token {
    pub fn new(kind: TokenKind, start: isize, end: isize) -> Self {
        Self::with_size(kind, start, end, 0)
    }

    pub fn with_size(kind: TokenKind, start: isize, end: isize, size: usize) -> Self {
        Self {
            kind,
            start,
            end,
            size,
        }
    }

    pub fn as_range(&self) -> Option<Range<usize>> {
        if self.start >= 0 && self.end >= 0 {
            Some(self.start as usize..self.end as usize)
        } else {
            None
        }
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
    /// List parsing
    ParseList,
}

pub struct BenDecoder {
    pos: usize,
    tok_next: usize,
    tok_super: isize,
}

impl Default for BenDecoder {
    fn default() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: -1,
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
                    if self.tok_super >= 0 {
                        tokens[self.tok_super as usize].size += 1;
                    }
                    self.pos += 1;
                    let start = self.pos;
                    self.parse_int(buf, b'e')?;
                    match self.alloc_token(tokens) {
                        Some(i) => {
                            tokens[i] =
                                Token::new(TokenKind::Int, start as isize, self.pos as isize);
                        }
                        None => {
                            self.pos = start;
                            return Err(Error::Invalid);
                        }
                    }
                    self.pos += 1;
                }
                b'l' => {
                    count += 1;
                    self.pos += 1;
                    if self.tok_super >= 0 {
                        let t = &mut tokens[self.tok_super as usize];
                        if t.kind == TokenKind::Dict {
                            return Err(Error::ParseList);
                        }
                        t.size += 1;
                    }
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    tokens[i] = Token::new(TokenKind::List, self.pos as isize, -1);
                    self.tok_super = self.tok_next as isize - 1;
                }
                b'd' => {
                    count += 1;
                    self.pos += 1;
                    if self.tok_super >= 0 {
                        tokens[self.tok_super as usize].size += 1;
                    }
                    let i = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    tokens[i] = Token::new(TokenKind::Dict, self.pos as isize, -1);
                    self.tok_super = self.tok_next as isize - 1;
                }
                b'0'..=b'9' => {
                    count += 1;
                    self.parse_string(buf, tokens)?;
                    if self.tok_super >= 0 {
                        let t = &mut tokens[self.tok_super as usize];
                        t.size += 1;
                        match t.kind {
                            TokenKind::Dict => self.tok_super = self.tok_next as isize - 1,
                            TokenKind::ByteStr => {
                                for i in (0..self.tok_next).rev() {
                                    let t = &tokens[i as usize];
                                    if let TokenKind::Dict = t.kind {
                                        if t.start >= 0 && t.end < 0 {
                                            self.tok_super = i as isize;
                                            break;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                b'e' => {
                    let mut i = (self.tok_next - 1) as isize;
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start >= 0 && token.end < 0 {
                            self.tok_super = -1;
                            token.end = self.pos as isize;
                            break;
                        } else {
                            i -= 1
                        }
                    }

                    // Error if unmatched closing bracket
                    if i == -1 {
                        return Err(Error::Invalid);
                    }

                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start >= 0 && token.end < 0 {
                            self.tok_super = i;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                    self.pos += 1;
                }
                _ => {
                    // Unexpected char
                    return Err(Error::Invalid);
                }
            }
        }
        for i in (0..self.tok_next).rev() {
            // Unclosed object
            if tokens[i].start >= 0 && tokens[i].end < 0 {
                return Err(Error::Part);
            }
        }
        Ok(count)
    }

    /// Parse bencode int.
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
                c => {
                    if c == stop_char {
                        break;
                    } else {
                        self.pos = start;
                        return Err(Error::Invalid);
                    }
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

        if let Some(i) = self.alloc_token(tokens) {
            tokens[i] = Token::new(
                TokenKind::ByteStr,
                self.pos as isize,
                (self.pos + len) as isize,
            );
            self.pos += len;
            Ok(())
        } else {
            self.pos = start;
            Err(Error::NoMemory)
        }
    }

    /// Allocates a fresh unused token from the token pool.
    fn alloc_token(&mut self, tokens: &mut [Token]) -> Option<usize> {
        if self.tok_next >= tokens.len() {
            return None;
        }
        let idx = self.tok_next as usize;
        self.tok_next += 1;
        let tok = &mut tokens[idx];
        tok.start = -1;
        tok.end = -1;
        tok.size = 0;
        Some(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! parse {
        ($buf: expr, $len: expr) => {{
            let mut v = [Token::default(); $len];
            let mut parser = BenDecoder::new();
            parser.parse($buf, &mut v).map(|parsed| {
                assert_eq!($len, parsed as usize);
                v
            })
        }};
    }

    #[test]
    fn parse_int() {
        let s = b"i12e";
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(&[Token::new(TokenKind::Int, 1, 3)], &tokens);
    }

    #[test]
    fn parse_string() {
        let s = b"3:abc";
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(&[Token::new(TokenKind::ByteStr, 2, 5)], &tokens);
    }

    #[test]
    fn parse_string_too_long() {
        let s = b"3:abcd";
        let err = parse!(s, 2).unwrap_err();
        assert_eq!(Error::Part, err);
    }

    #[test]
    fn parse_string_too_short() {
        let s = b"3:ab";
        let err = parse!(s, 2).unwrap_err();
        assert_eq!(Error::ParseStr, err);
    }

    #[test]
    fn empty_dict() {
        let s = b"de";
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(&[Token::new(TokenKind::Dict, 1, 1)], &tokens);
    }

    #[test]
    fn unclosed_dict() {
        let s = b"d";
        let err = parse!(s, 1).unwrap_err();
        assert_eq!(Error::Part, err);
    }

    #[test]
    fn dict_string_values() {
        let s = b"d1:a2:ab3:abc4:abcde";
        let tokens = parse!(s, 5).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Dict, 1, 19, 2),
                Token::with_size(TokenKind::ByteStr, 3, 4, 1),
                Token::with_size(TokenKind::ByteStr, 6, 8, 0),
                Token::with_size(TokenKind::ByteStr, 10, 13, 1),
                Token::with_size(TokenKind::ByteStr, 15, 19, 0)
            ],
            &tokens
        );
    }

    #[test]
    fn empty_list() {
        let s = b"le";
        let tokens = parse!(s, 1).unwrap();
        assert_eq!(&[Token::new(TokenKind::List, 1, 1)], &tokens);
    }

    #[test]
    fn unclosed_list() {
        let s = b"l";
        let err = parse!(s, 1).unwrap_err();
        assert_eq!(Error::Part, err);
    }

    #[test]
    fn list_string_values() {
        let s = b"l1:a2:ab3:abc4:abcde";
        let tokens = parse!(s, 5).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 1, 19, 4),
                Token::new(TokenKind::ByteStr, 3, 4),
                Token::new(TokenKind::ByteStr, 6, 8),
                Token::new(TokenKind::ByteStr, 10, 13),
                Token::new(TokenKind::ByteStr, 15, 19)
            ],
            &tokens
        );
    }

    #[test]
    fn list_nested() {
        let s = b"lllleeee";
        let tokens = parse!(s, 4).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 1, 7, 1),
                Token::with_size(TokenKind::List, 2, 6, 1),
                Token::with_size(TokenKind::List, 3, 5, 1),
                Token::with_size(TokenKind::List, 4, 4, 0),
            ],
            &tokens
        );
    }

    #[test]
    fn list_nested_complex() {
        let s = b"ld1:ald2:ablleeeeee";
        let tokens = parse!(s, 8).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 1, 18, 1),
                Token::with_size(TokenKind::Dict, 2, 17, 1),
                Token::with_size(TokenKind::ByteStr, 4, 5, 1),
                Token::with_size(TokenKind::List, 6, 16, 1),
                Token::with_size(TokenKind::Dict, 7, 15, 1),
                Token::with_size(TokenKind::ByteStr, 9, 11, 1),
                Token::with_size(TokenKind::List, 12, 14, 1),
                Token::with_size(TokenKind::List, 13, 13, 0),
            ],
            &tokens
        );
    }
}
