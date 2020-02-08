//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.

#![no_std]

use core::ops::Range;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    None,
    Dict,
    List,
    ByteStr,
    Int,
    End,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: isize,
    pub end: isize,
    pub children: usize,
}

impl Default for Token {
    fn default() -> Self {
        Self::new(TokenKind::None, -1, -1)
    }
}

impl Token {
    pub fn new(kind: TokenKind, start: isize, end: isize) -> Self {
        Self::with_size(kind, start, end, 0)
    }

    pub fn with_size(kind: TokenKind, start: isize, end: isize, children: usize) -> Self {
        Self {
            kind,
            start,
            end,
            children,
        }
    }

    pub fn range(&self) -> Range<usize> {
        assert!(self.start >= 0);
        assert!(self.end >= self.start);

        self.start as usize..self.end as usize
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

/// Bencode Decoder
pub struct BenDecoder {
    pos: usize,
    next: usize,
    parent: isize,
}

impl Default for BenDecoder {
    fn default() -> Self {
        Self {
            pos: 0,
            next: 0,
            parent: -1,
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
        let mut count = self.next;
        while self.pos < buf.len() {
            let c = buf[self.pos];
            match c {
                b'i' => {
                    count += 1;
                    self.update_super(tokens, TokenKind::Int)?;
                    self.pos += 1;
                    let start = self.pos;
                    self.parse_int(buf, b'e')?;
                    match self.alloc_token(tokens) {
                        Some(token) => {
                            *token = Token::new(TokenKind::Int, start as isize, self.pos as isize);
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
                    self.update_super(tokens, TokenKind::List)?;
                    let token = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    *token = Token::new(TokenKind::List, self.pos as isize, -1);
                    self.parent = self.next as isize - 1;
                }
                b'd' => {
                    count += 1;
                    self.pos += 1;
                    self.update_super(tokens, TokenKind::Dict)?;
                    let token = self.alloc_token(tokens).ok_or(Error::NoMemory)?;
                    *token = Token::new(TokenKind::Dict, self.pos as isize, -1);
                    self.parent = self.next as isize - 1;
                }
                b'0'..=b'9' => {
                    count += 1;
                    self.parse_string(buf, tokens)?;
                    self.update_super(tokens, TokenKind::ByteStr)?;
                }
                b'e' => {
                    let mut i = (self.next - 1) as isize;
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start >= 0 && token.end < 0 {
                            self.parent = -1;
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
                            self.parent = i;
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
        for i in (0..self.next).rev() {
            // Unclosed object
            if tokens[i].start >= 0 && tokens[i].end < 0 {
                return Err(Error::Part);
            }
        }
        Ok(count)
    }

    fn update_super(&mut self, tokens: &mut [Token], my_kind: TokenKind) -> Result<(), Error> {
        if self.parent >= 0 {
            let t = &mut tokens[self.parent as usize];
            t.children += 1;
            match t.kind {
                TokenKind::Dict => {
                    if let TokenKind::ByteStr = my_kind {
                        self.parent = self.next as isize - 1;
                    } else {
                        // Can't have key other than byte string
                        return Err(Error::Invalid);
                    }
                }
                TokenKind::ByteStr => {
                    for i in (0..self.next).rev() {
                        let t = &tokens[i as usize];
                        if let TokenKind::Dict = t.kind {
                            if t.start >= 0 && t.end < 0 {
                                self.parent = i as isize;
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
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

        if let Some(token) = self.alloc_token(tokens) {
            *token = Token::new(
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
    fn alloc_token<'a>(&mut self, tokens: &'a mut [Token]) -> Option<&'a mut Token> {
        if self.next >= tokens.len() {
            return None;
        }
        let token = &mut tokens[self.next as usize];
        self.next += 1;
        Some(token)
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
                assert_eq!($len, parsed);
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
    fn dict_mixed_values() {
        let s = b"d1:a1:b1:ci1e1:x1:y1:dde1:fle1:g1:he";
        let tokens = parse!(s, 13).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Dict, 1, 35, 6),
                Token::with_size(TokenKind::ByteStr, 3, 4, 1),
                Token::with_size(TokenKind::ByteStr, 6, 7, 0),
                Token::with_size(TokenKind::ByteStr, 9, 10, 1),
                Token::with_size(TokenKind::Int, 11, 12, 0),
                Token::with_size(TokenKind::ByteStr, 15, 16, 1),
                Token::with_size(TokenKind::ByteStr, 18, 19, 0),
                Token::with_size(TokenKind::ByteStr, 21, 22, 1),
                Token::with_size(TokenKind::Dict, 23, 23, 0),
                Token::with_size(TokenKind::ByteStr, 26, 27, 1),
                Token::with_size(TokenKind::List, 28, 28, 0),
                Token::with_size(TokenKind::ByteStr, 31, 32, 1),
                Token::with_size(TokenKind::ByteStr, 34, 35, 0)
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
