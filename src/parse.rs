use crate::Node;
use std::fmt;
use std::ops::Range;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    Dict,
    List,
    ByteStr,
    Int,
}

#[derive(Clone, PartialEq)]
pub struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) start: i32,
    pub(crate) end: i32,
    pub(crate) children: u32,
    pub(crate) next: u32,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}[{}:{}]", self.kind, self.start, self.end)
    }
}

impl Token {
    pub(crate) fn new(kind: TokenKind, start: i32, end: i32) -> Self {
        Self::with_size(kind, start, end, 0, 1)
    }

    pub(crate) fn with_size(
        kind: TokenKind,
        start: i32,
        end: i32,
        children: u32,
        next: u32,
    ) -> Self {
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
    Eof,
    /// Invalid character inside Bencode string
    Unexpected { pos: usize },
    /// Invalid character inside Bencode string
    Invalid { reason: &'static str, pos: usize },
    /// Not enough tokens were provided
    NoMemory,
    /// Integer Overflow
    Overflow { pos: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Eof => write!(f, "Unexpected End of File"),
            Self::Unexpected { pos } => write!(f, "Unexpected character at {}", pos),
            Self::Invalid { reason, pos } => write!(f, "Invalid input at {}: {}", pos, reason),
            Self::NoMemory => write!(f, "No tokens left to parse"),
            Self::Overflow { pos } => write!(f, "Integer overflow at {}", pos),
        }
    }
}

impl std::error::Error for Error {}

/// Bencode Parser
pub struct Parser {
    pos: usize,
    tok_next: usize,
    tok_super: isize,
    token_limit: usize,
    tokens: Vec<Token>,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            pos: 0,
            tok_next: 0,
            tok_super: -1,
            token_limit: usize::max_value(),
            tokens: vec![],
        }
    }
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_token_limit(&mut self, token_limit: usize) {
        self.token_limit = token_limit;
    }

    /// Run Bencode parser. It parses a bencoded data string and returns a vector of tokens, each
    /// describing a single Bencode object.
    pub fn parse<'a>(&'a mut self, buf: &'a [u8]) -> Result<Node<'a>, Error> {
        let (node, len) = self.parse_prefix(buf)?;
        if len == buf.len() {
            Ok(node)
        } else {
            Err(Error::Invalid {
                reason: "Extra bytes at the end",
                pos: len,
            })
        }
    }

    /// Run Bencode parser. It parses a bencoded data string into given vector of tokens, each
    /// describing a single Bencode object.
    pub fn parse_prefix<'a>(&'a mut self, buf: &'a [u8]) -> Result<(Node<'a>, usize), Error> {
        if buf.is_empty() {
            return Err(Error::Eof);
        }

        self.reset();
        let mut depth = 0;
        while self.pos < buf.len() {
            let c = buf[self.pos];
            match c {
                b'i' => {
                    self.update_super(TokenKind::Int)?;
                    self.pos += 1;
                    let start = self.pos;
                    self.parse_int(buf, b'e')?;
                    let token = Token::new(TokenKind::Int, start as _, self.pos as _);
                    if let Err(e) = self.alloc_token(token) {
                        self.pos = start;
                        return Err(e);
                    }
                    self.pos += 1;
                }
                b'l' => {
                    depth += 1;
                    let token = Token::new(TokenKind::List, self.pos as _, -1);
                    self.pos += 1;
                    self.alloc_token(token)?;
                    self.update_super(TokenKind::List)?;
                    self.tok_super = self.tok_next as isize - 1;
                }
                b'd' => {
                    depth += 1;
                    let token = Token::new(TokenKind::Dict, self.pos as _, -1);
                    self.pos += 1;
                    self.alloc_token(token)?;
                    self.update_super(TokenKind::Dict)?;
                    self.tok_super = self.tok_next as isize - 1;
                }
                b'0'..=b'9' => {
                    self.parse_string(buf)?;
                    self.update_super(TokenKind::ByteStr)?;
                }
                b'e' => {
                    self.pos += 1;
                    depth -= 1;
                    let mut i = (self.tok_next - 1) as i32;
                    while i >= 0 {
                        let token = &mut self.tokens[i as usize];
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
                        return Err(Error::Invalid {
                            reason: "Unclosed object",
                            pos: self.pos,
                        });
                    }

                    while i >= 0 {
                        let token = &self.tokens[i as usize];
                        if token.start >= 0 && token.end < 0 {
                            self.tok_super = i as _;
                            break;
                        } else {
                            i -= 1
                        }
                    }
                }
                _ => {
                    // Unexpected char
                    return Err(Error::Unexpected { pos: self.pos });
                }
            }
            if depth == 0 {
                break;
            }
        }
        for i in (0..self.tok_next).rev() {
            let token = &self.tokens[i];

            // Unclosed object
            if token.start >= 0 && token.end < 0 {
                return Err(Error::Eof);
            }

            if let TokenKind::Dict = token.kind {
                if token.children % 2 != 0 {
                    return Err(Error::Eof);
                }
            }
        }
        let node = Node {
            buf,
            tokens: &self.tokens,
            idx: 0,
        };
        Ok((node, self.pos))
    }

    fn reset(&mut self) {
        self.tokens.clear();
        self.pos = 0;
        self.tok_next = 0;
        self.tok_super = -1;
    }

    fn update_super(&mut self, curr_kind: TokenKind) -> Result<(), Error> {
        if self.tok_super < 0 {
            return Ok(());
        }

        let t = &mut self.tokens[self.tok_super as usize];
        t.children += 1;
        if let TokenKind::Dict = t.kind {
            if curr_kind != TokenKind::ByteStr && t.children % 2 != 0 {
                return Err(Error::Invalid {
                    reason: "Dictionary key must be a string",
                    pos: self.pos,
                });
            }
        }
        Ok(())
    }

    /// Parse bencode int.
    fn parse_int(&mut self, buf: &[u8], stop_char: u8) -> Result<i64, Error> {
        if self.pos >= buf.len() {
            return Err(Error::Eof);
        }

        let mut negative = false;
        let mut val = 0;

        let start = self.pos;

        if buf[self.pos] == b'-' {
            self.pos += 1;
            negative = true;
            if self.pos == buf.len() {
                self.pos = start;
                return Err(Error::Eof);
            }
        }

        while self.pos < buf.len() {
            match buf[self.pos] {
                c @ b'0'..=b'9' => {
                    if val > i64::max_value() / 10 {
                        self.pos = start;
                        return Err(Error::Overflow { pos: start });
                    }
                    val *= 10;
                    let digit = (c - b'0') as i64;
                    if val > i64::max_value() - digit {
                        self.pos = start;
                        return Err(Error::Overflow { pos: start });
                    }
                    val += digit;
                    self.pos += 1
                }
                c => {
                    if c == stop_char {
                        break;
                    } else {
                        let pos = self.pos;
                        self.pos = start;
                        return Err(Error::Unexpected { pos });
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
    fn parse_string(&mut self, buf: &[u8]) -> Result<(), Error> {
        let start = self.pos;

        let len = self.parse_int(buf, b':')?;
        self.pos += 1; // Skip the ':'

        if len < 0 {
            self.pos = start;
            return Err(Error::Invalid {
                reason: "String length must be positive",
                pos: self.pos,
            });
        }

        let len = len as usize;
        if self.pos + len > buf.len() {
            self.pos = start;
            return Err(Error::Eof);
        }

        let token = Token::new(TokenKind::ByteStr, self.pos as _, (self.pos + len) as _);
        if let Ok(_) = self.alloc_token(token) {
            self.pos += len;
            Ok(())
        } else {
            self.pos = start;
            Err(Error::NoMemory)
        }
    }

    /// Returns the next unused token from the slice.
    fn alloc_token(&mut self, token: Token) -> Result<(), Error> {
        if self.tokens.len() >= self.token_limit {
            return Err(Error::NoMemory);
        }
        self.tokens.push(token);
        self.tok_next += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int() {
        let s = b"i12e";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::Int, 1, 3)], &parser.tokens[..]);
    }

    #[test]
    fn parse_string() {
        let s = b"3:abc";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::ByteStr, 2, 5)], &parser.tokens[..]);
    }

    #[test]
    fn parse_string_too_long() {
        let s = b"3:abcd";
        let mut parser = Parser::new();
        let err = parser.parse(s).unwrap_err();
        assert_eq!(
            Error::Invalid {
                reason: "Extra bytes at the end",
                pos: 5,
            },
            err
        );
    }

    #[test]
    fn parse_string_too_short() {
        let s = b"3:ab";
        let mut parser = Parser::new();
        let err = parser.parse(s).unwrap_err();
        assert_eq!(Error::Eof, err);
    }

    #[test]
    fn empty_dict() {
        let s = b"de";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::Dict, 0, 2)], &parser.tokens[..]);
    }

    #[test]
    fn unclosed_dict() {
        let s = b"d";
        let err = Parser::new().parse(s).unwrap_err();
        assert_eq!(Error::Eof, err);
    }

    #[test]
    fn key_only_dict() {
        let s = b"d1:ae";
        let err = Parser::new().parse(s).unwrap_err();
        assert_eq!(Error::Eof, err);
    }

    #[test]
    fn key_only_dict_2() {
        let s = b"d1:a1:a1:ae";
        let err = Parser::new().parse(s).unwrap_err();
        assert_eq!(Error::Eof, err);
    }

    #[test]
    fn dict_string_values() {
        let s = b"d1:a2:ab3:abc4:abcde";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Dict, 0, 20, 4, 5),
                Token::with_size(TokenKind::ByteStr, 3, 4, 0, 1),
                Token::with_size(TokenKind::ByteStr, 6, 8, 0, 1),
                Token::with_size(TokenKind::ByteStr, 10, 13, 0, 1),
                Token::with_size(TokenKind::ByteStr, 15, 19, 0, 1)
            ],
            &parser.tokens[..]
        );
    }

    #[test]
    fn dict_mixed_values() {
        let s = b"d1:a1:b1:ci1e1:x1:y1:dde1:fle1:g1:he";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::Dict, 0, 36, 12, 13),
                Token::with_size(TokenKind::ByteStr, 3, 4, 0, 1),
                Token::with_size(TokenKind::ByteStr, 6, 7, 0, 1),
                Token::with_size(TokenKind::ByteStr, 9, 10, 0, 1),
                Token::with_size(TokenKind::Int, 11, 12, 0, 1),
                Token::with_size(TokenKind::ByteStr, 15, 16, 0, 1),
                Token::with_size(TokenKind::ByteStr, 18, 19, 0, 1),
                Token::with_size(TokenKind::ByteStr, 21, 22, 0, 1),
                Token::with_size(TokenKind::Dict, 22, 24, 0, 1),
                Token::with_size(TokenKind::ByteStr, 26, 27, 0, 1),
                Token::with_size(TokenKind::List, 27, 29, 0, 1),
                Token::with_size(TokenKind::ByteStr, 31, 32, 0, 1),
                Token::with_size(TokenKind::ByteStr, 34, 35, 0, 1)
            ],
            &parser.tokens[..]
        );
    }

    #[test]
    fn empty_list() {
        let s = b"le";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(&[Token::new(TokenKind::List, 0, 2)], &parser.tokens[..]);
    }

    #[test]
    fn unclosed_list() {
        let s = b"l";
        let err = Parser::new().parse(s).unwrap_err();
        assert_eq!(Error::Eof, err);
    }

    #[test]
    fn list_string_values() {
        let s = b"l1:a2:ab3:abc4:abcde";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 0, 20, 4, 5),
                Token::new(TokenKind::ByteStr, 3, 4),
                Token::new(TokenKind::ByteStr, 6, 8,),
                Token::new(TokenKind::ByteStr, 10, 13,),
                Token::new(TokenKind::ByteStr, 15, 19,)
            ],
            &parser.tokens[..]
        );
    }

    #[test]
    fn list_nested() {
        let s = b"lllleeee";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 0, 8, 1, 4),
                Token::with_size(TokenKind::List, 1, 7, 1, 3),
                Token::with_size(TokenKind::List, 2, 6, 1, 2),
                Token::with_size(TokenKind::List, 3, 5, 0, 1),
            ],
            &parser.tokens[..]
        );
    }

    #[test]
    fn list_nested_complex() {
        let s = b"ld1:ald2:ablleeeeee";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(
            &[
                Token::with_size(TokenKind::List, 0, 19, 1, 8),
                Token::with_size(TokenKind::Dict, 1, 18, 2, 7),
                Token::with_size(TokenKind::ByteStr, 4, 5, 0, 1),
                Token::with_size(TokenKind::List, 5, 17, 1, 5),
                Token::with_size(TokenKind::Dict, 6, 16, 2, 4),
                Token::with_size(TokenKind::ByteStr, 9, 11, 0, 1),
                Token::with_size(TokenKind::List, 11, 15, 1, 2),
                Token::with_size(TokenKind::List, 12, 14, 0, 1),
            ],
            &parser.tokens[..]
        );
    }

    #[test]
    fn token_limit() {
        let s = b"l1:a2:ab3:abc4:abcde";
        let mut parser = Parser::new();
        parser.set_token_limit(3);
        let err = parser.parse(s).unwrap_err();
        assert_eq!(Error::NoMemory, err);
    }

    #[test]
    fn multiple_root_tokens() {
        let mut parser = Parser::new();
        assert_eq!(
            Error::Invalid {
                reason: "Extra bytes at the end",
                pos: 3,
            },
            parser.parse(b"1:a1:b").unwrap_err()
        );
        assert_eq!(
            Error::Invalid {
                reason: "Extra bytes at the end",
                pos: 3,
            },
            parser.parse(b"i1e1:b").unwrap_err()
        );
        assert_eq!(
            Error::Invalid {
                reason: "Extra bytes at the end",
                pos: 5,
            },
            parser.parse(b"l1:aede").unwrap_err()
        );
        assert_eq!(
            Error::Invalid {
                reason: "Extra bytes at the end",
                pos: 2,
            },
            parser.parse(b"lel1:ae").unwrap_err()
        );
    }

    #[test]
    fn parse_prefix() {
        let s = b"lede";
        let mut parser = Parser::new();
        let (_, len) = parser.parse_prefix(s).unwrap();
        assert_eq!(
            &[Token::with_size(TokenKind::List, 0, 2, 0, 1)],
            &parser.tokens[..]
        );
        assert_eq!(2, len);
    }

    #[test]
    fn parse_prefix_in() {
        let s = b"lede";
        let mut parser = Parser::new();
        let (_, len) = parser.parse_prefix(s).unwrap();
        assert_eq!(
            &[Token::with_size(TokenKind::List, 0, 2, 0, 1)],
            &parser.tokens[..]
        );
        assert_eq!(2, len);
    }

    #[test]
    fn parse_empty_string() {
        let s = b"0:";
        let mut parser = Parser::new();
        parser.parse(s).unwrap();
        assert_eq!(
            &[Token::with_size(TokenKind::ByteStr, 2, 2, 0, 1)],
            &parser.tokens[..]
        );
    }
}
