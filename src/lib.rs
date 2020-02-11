//! `ben` is an efficient Bencode parser which parses the structure into
//! a flat stream of tokens rather than an actual tree and thus avoids
//! unneccessary allocations.
pub mod node;

use std::ops::Range;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    None,
    Dict,
    List,
    ByteStr,
    Int,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) start: i32,
    pub(crate) end: i32,
    pub(crate) children: u32,
    pub(crate) next: u32,
}

impl Default for Token {
    fn default() -> Self {
        Self::new(TokenKind::None, -1, -1)
    }
}

impl Token {
    pub fn new(kind: TokenKind, start: i32, end: i32) -> Self {
        Self::with_size(kind, start, end, 0, 1)
    }

    pub fn with_size(kind: TokenKind, start: i32, end: i32, children: u32, next: u32) -> Self {
        Self {
            kind,
            start,
            end,
            children,
            next,
        }
    }

    /// Returns this token's bounds in the original buffer.
    ///
    /// # Panics
    /// If the token is not valid
    pub fn range(&self) -> Range<usize> {
        assert!(self.start >= 0);
        assert!(self.end >= self.start);

        self.start as usize..self.end as usize
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Error {
    /// The string is not a full Bencode packet, more bytes expected
    Incomplete,
    /// Invalid character inside Bencode string
    Invalid,
    /// Not enough tokens were provided
    NoMemory,
    /// Overflow (numeric or otherwise)
    Overflow,
}

/// Bencode Decoder
pub struct BenDecoder {
    pos: usize,
    tok_next: usize,
    tok_super: isize,
    token_limit: usize,
}

impl Default for BenDecoder {
    fn default() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: -1,
            token_limit: usize::max_value(),
        }
    }
}

impl BenDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_token_limit(&mut self, token_limit: usize) {
        self.token_limit = token_limit;
    }

    /// Run Bencode parser. It parses a bencoded data string and returns a vector of tokens, each
    /// describing a single Bencode object.
    pub fn parse(&mut self, buf: &[u8]) -> Result<Vec<Token>, Error> {
        let mut tokens = vec![];
        self.parse_in(buf, &mut tokens)?;
        Ok(tokens)
    }

    /// Run Bencode parser. It parses a bencoded data string into given vector of tokens, each
    /// describing a single Bencode object.
    pub fn parse_in(&mut self, buf: &[u8], tokens: &mut Vec<Token>) -> Result<(), Error> {
        tokens.clear();
        while self.pos < buf.len() {
            let c = buf[self.pos];
            match c {
                b'i' => {
                    self.update_super(TokenKind::Int, tokens)?;
                    self.pos += 1;
                    let start = self.pos;
                    self.parse_int(buf, b'e')?;
                    let token = Token::new(TokenKind::Int, start as _, self.pos as _);
                    if let Err(e) = self.alloc_token(token, tokens) {
                        self.pos = start;
                        return Err(e);
                    }
                    self.pos += 1;
                }
                b'l' => {
                    self.pos += 1;
                    let token = Token::new(TokenKind::List, self.pos as _, -1);
                    self.alloc_token(token, tokens)?;
                    self.update_super(TokenKind::List, tokens)?;
                    self.tok_super = self.tok_next as isize - 1;
                }
                b'd' => {
                    self.pos += 1;
                    let token = Token::new(TokenKind::Dict, self.pos as _, -1);
                    self.alloc_token(token, tokens)?;
                    self.update_super(TokenKind::Dict, tokens)?;
                    self.tok_super = self.tok_next as isize - 1;
                }
                b'0'..=b'9' => {
                    self.parse_string(buf, tokens)?;
                    self.update_super(TokenKind::ByteStr, tokens)?;
                }
                b'e' => {
                    let mut i = (self.tok_next - 1) as i32;
                    while i >= 0 {
                        let token = &mut tokens[i as usize];
                        if token.start >= 0 && token.end < 0 {
                            token.next = self.tok_next as u32 - i as u32;
                            self.tok_super = -1;
                            token.end = self.pos as _;
                            break;
                        } else {
                            i -= 1
                        }
                    }

                    // Error if unclosed object
                    if i == -1 {
                        return Err(Error::Invalid);
                    }

                    while i >= 0 {
                        let token = tokens[i as usize];
                        if token.start >= 0 && token.end < 0 {
                            self.tok_super = i as _;
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
            let token = &tokens[i];

            // Unclosed object
            if token.start >= 0 && token.end < 0 {
                return Err(Error::Incomplete);
            }

            if let TokenKind::Dict = token.kind {
                if token.children % 2 != 0 {
                    return Err(Error::Incomplete);
                }
            }
        }
        Ok(())
    }

    fn update_super(&mut self, curr_kind: TokenKind, tokens: &mut [Token]) -> Result<(), Error> {
        if self.tok_super < 0 {
            return Ok(());
        }

        let t = &mut tokens[self.tok_super as usize];
        t.children += 1;
        if let TokenKind::Dict = t.kind {
            if curr_kind != TokenKind::ByteStr && t.children % 2 != 0 {
                return Err(Error::Invalid); // Can't have key other than byte string
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
    fn parse_string(&mut self, buf: &[u8], tokens: &mut Vec<Token>) -> Result<(), Error> {
        let start = self.pos;

        let len = self.parse_int(buf, b':')?;
        self.pos += 1; // Skip the ':'

        if len <= 0 {
            self.pos = start;
            return Err(Error::Invalid);
        }

        let len = len as usize;
        if self.pos + len > buf.len() {
            self.pos = start;
            return Err(Error::Invalid);
        }

        let token = Token::new(TokenKind::ByteStr, self.pos as _, (self.pos + len) as _);
        if let Ok(_) = self.alloc_token(token, tokens) {
            self.pos += len;
            Ok(())
        } else {
            self.pos = start;
            Err(Error::NoMemory)
        }
    }

    /// Returns the next unused token from the slice.
    fn alloc_token(&mut self, token: Token, tokens: &mut Vec<Token>) -> Result<(), Error> {
        if tokens.len() >= self.token_limit {
            return Err(Error::NoMemory);
        }
        tokens.push(token);
        self.tok_next += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! parse {
        ($buf: expr) => {{
            let mut parser = BenDecoder::new();
            parser.parse($buf)
        }};
    }

    #[test]
    fn parse_int() {
        let s = b"i12e";
        let tokens = parse!(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::Int, 1, 3)], &tokens[..]);
    }

    #[test]
    fn parse_string() {
        let s = b"3:abc";
        let tokens = parse!(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::ByteStr, 2, 5)], &tokens[..]);
    }

    #[test]
    fn parse_string_too_long() {
        let s = b"3:abcd";
        let err = parse!(s).unwrap_err();
        assert_eq!(Error::Incomplete, err);
    }

    #[test]
    fn parse_string_too_short() {
        let s = b"3:ab";
        let err = parse!(s).unwrap_err();
        assert_eq!(Error::Invalid, err);
    }

    #[test]
    fn empty_dict() {
        let s = b"de";
        let tokens = parse!(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::Dict, 1, 1)], &tokens[..]);
    }

    #[test]
    fn unclosed_dict() {
        let s = b"d";
        let err = parse!(s).unwrap_err();
        assert_eq!(Error::Incomplete, err);
    }

    #[test]
    fn key_only_dict() {
        let s = b"d1:ae";
        let err = parse!(s).unwrap_err();
        assert_eq!(Error::Incomplete, err);
    }

    #[test]
    fn key_only_dict_2() {
        let s = b"d1:a1:a1:ae";
        let err = parse!(s).unwrap_err();
        assert_eq!(Error::Incomplete, err);
    }

    #[test]
    fn dict_string_values() {
        let s = b"d1:a2:ab3:abc4:abcde";
        let tokens = parse!(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Dict, 1, 19, 4, 5),
                Token::with_size(TokenKind::ByteStr, 3, 4, 0, 1),
                Token::with_size(TokenKind::ByteStr, 6, 8, 0, 1),
                Token::with_size(TokenKind::ByteStr, 10, 13, 0, 1),
                Token::with_size(TokenKind::ByteStr, 15, 19, 0, 1)
            ],
            &tokens[..]
        );
    }

    #[test]
    fn dict_mixed_values() {
        let s = b"d1:a1:b1:ci1e1:x1:y1:dde1:fle1:g1:he";
        let tokens = parse!(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Dict, 1, 35, 12, 13),
                Token::with_size(TokenKind::ByteStr, 3, 4, 0, 1),
                Token::with_size(TokenKind::ByteStr, 6, 7, 0, 1),
                Token::with_size(TokenKind::ByteStr, 9, 10, 0, 1),
                Token::with_size(TokenKind::Int, 11, 12, 0, 1),
                Token::with_size(TokenKind::ByteStr, 15, 16, 0, 1),
                Token::with_size(TokenKind::ByteStr, 18, 19, 0, 1),
                Token::with_size(TokenKind::ByteStr, 21, 22, 0, 1),
                Token::with_size(TokenKind::Dict, 23, 23, 0, 1),
                Token::with_size(TokenKind::ByteStr, 26, 27, 0, 1),
                Token::with_size(TokenKind::List, 28, 28, 0, 1),
                Token::with_size(TokenKind::ByteStr, 31, 32, 0, 1),
                Token::with_size(TokenKind::ByteStr, 34, 35, 0, 1)
            ],
            &tokens[..]
        );
    }

    #[test]
    fn empty_list() {
        let s = b"le";
        let tokens = parse!(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::List, 1, 1)], &tokens[..]);
    }

    #[test]
    fn unclosed_list() {
        let s = b"l";
        let err = parse!(s).unwrap_err();
        assert_eq!(Error::Incomplete, err);
    }

    #[test]
    fn list_string_values() {
        let s = b"l1:a2:ab3:abc4:abcde";
        let tokens = parse!(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 1, 19, 4, 5),
                Token::new(TokenKind::ByteStr, 3, 4),
                Token::new(TokenKind::ByteStr, 6, 8,),
                Token::new(TokenKind::ByteStr, 10, 13,),
                Token::new(TokenKind::ByteStr, 15, 19,)
            ],
            &tokens[..]
        );
    }

    #[test]
    fn list_nested() {
        let s = b"lllleeee";
        let tokens = parse!(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 1, 7, 1, 4),
                Token::with_size(TokenKind::List, 2, 6, 1, 3),
                Token::with_size(TokenKind::List, 3, 5, 1, 2),
                Token::with_size(TokenKind::List, 4, 4, 0, 1),
            ],
            &tokens[..]
        );
    }

    #[test]
    fn list_nested_complex() {
        let s = b"ld1:ald2:ablleeeeee";
        let tokens = parse!(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 1, 18, 1, 8),
                Token::with_size(TokenKind::Dict, 2, 17, 2, 7),
                Token::with_size(TokenKind::ByteStr, 4, 5, 0, 1),
                Token::with_size(TokenKind::List, 6, 16, 1, 5),
                Token::with_size(TokenKind::Dict, 7, 15, 2, 4),
                Token::with_size(TokenKind::ByteStr, 9, 11, 0, 1),
                Token::with_size(TokenKind::List, 12, 14, 1, 2),
                Token::with_size(TokenKind::List, 13, 13, 0, 1),
            ],
            &tokens[..]
        );
    }

    #[test]
    fn token_limit() {
        let s = b"l1:a2:ab3:abc4:abcde";
        let mut parser = BenDecoder::new();
        parser.set_token_limit(3);
        let err = parser.parse(s).unwrap_err();
        assert_eq!(Error::NoMemory, err);
    }
}
