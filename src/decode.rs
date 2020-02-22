use crate::parse::TokenKind;
use crate::parse::{Parser, Token};
use std::borrow::Cow;
use std::fmt;

#[derive(PartialEq)]
pub struct Node<'a> {
    buf: &'a [u8],
    tokens: Cow<'a, [Token]>,
    idx: usize,
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            TokenKind::Int => write!(f, "{}", self.as_int().unwrap()),
            TokenKind::ByteStr => match self.as_str() {
                Some(s) => write!(f, "{}", s),
                None => write!(f, "`Bytes:{:?}`", self.data()),
            },
            TokenKind::List => f
                .debug_list()
                .entries(self.as_list().unwrap().iter())
                .finish(),
            TokenKind::Dict => f
                .debug_map()
                .entries(self.as_dict().unwrap().iter())
                .finish(),
        }
    }
}

impl<'a> Node<'a> {
    /// Parse given bencoded bytes.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"11:Hello World";
    ///     let node = Node::parse(bytes).unwrap();
    ///     assert_eq!("Hello World", node.as_str().unwrap());
    /// ```
    pub fn parse(buf: &'a [u8]) -> crate::Result<Self> {
        Self::parse_max_tokens(buf, usize::max_value())
    }

    /// Parse given bencoded bytes using a given token buffer.
    /// It helps you reuse the buffer next time.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let mut v = vec![];
    ///     let values: &[&[u8]] = &[b"5:Hello", b"5:World"];
    ///     for bytes in values {
    ///         let node = Node::parse_in(bytes, &mut v).unwrap();
    ///         assert!(node.is_str());
    ///     }
    /// ```
    pub fn parse_in(buf: &'a [u8], tokens: &'a mut Vec<Token>) -> crate::Result<Self> {
        let parser = Parser::new();
        parser.parse_in(buf, tokens)?;
        Ok(Self {
            buf,
            tokens: Cow::Borrowed(tokens),
            idx: 0,
        })
    }

    /// Parse given bencoded bytes from the beginning and returns
    /// index where one root object is parsed successfully.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"5:Hello World";
    ///     let (node, i) = Node::parse_prefix(bytes).unwrap();
    ///     assert_eq!("Hello", node.as_str().unwrap());
    ///     assert_eq!(b" World", &bytes[i..]);
    /// ```
    pub fn parse_prefix(buf: &'a [u8]) -> crate::Result<(Self, usize)> {
        let parser = Parser::new();
        let (tokens, len) = parser.parse_prefix(buf)?;
        let node = Self {
            buf,
            tokens: Cow::Owned(tokens),
            idx: 0,
        };
        Ok((node, len))
    }

    /// Parse given bencoded bytes from the beginning and returns
    /// index where one root object is parsed successfully. It accepts
    /// a token buffer argument which helps reuse the buffer next time.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let mut v = vec![];
    ///     let values: &[&[u8]] = &[b"5:Hello World", b"1:ade"];
    ///     for bytes in values {
    ///         let (node, i) = Node::parse_prefix_in(bytes, &mut v).unwrap();
    ///         assert!(node.is_str());
    ///     }
    /// ```
    pub fn parse_prefix_in(
        buf: &'a [u8],
        tokens: &'a mut Vec<Token>,
    ) -> crate::Result<(Self, usize)> {
        let parser = Parser::new();
        let len = parser.parse_prefix_in(buf, tokens)?;
        let node = Self {
            buf,
            tokens: Cow::Borrowed(tokens),
            idx: 0,
        };
        Ok((node, len))
    }

    /// Parse given bencoded bytes with limit on maximum number of
    /// tokens that can be parsed.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::{Node, Error};
    ///
    ///     let bytes = b"l1:a2:bce";
    ///     let err = Node::parse_max_tokens(bytes, 2).unwrap_err();
    ///     assert_eq!(Error::NoMemory, err);
    /// ```
    pub fn parse_max_tokens(buf: &'a [u8], max_tokens: usize) -> crate::Result<Self> {
        let mut parser = Parser::new();
        parser.set_token_limit(max_tokens);
        Ok(Self {
            buf,
            tokens: Cow::Owned(parser.parse(buf)?),
            idx: 0,
        })
    }

    /// Returns raw bytes of this node.
    ///
    /// This returns complete raw bytes for dict and list, but remove the headers
    /// from string and int.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"l1:a2:bce";
    ///     let node = Node::parse(bytes).unwrap();
    ///     assert_eq!(b"l1:a2:bce", node.data());
    /// ```
    pub fn data(&self) -> &'a [u8] {
        &self.buf[self.tokens[self.idx].range()]
    }

    fn kind(&self) -> TokenKind {
        self.tokens[self.idx].kind
    }

    /// Returns true if this node is a list.
    pub fn is_list(&self) -> bool {
        self.kind() == TokenKind::List
    }

    /// Returns true if this node is a dictionary.
    pub fn is_dict(&self) -> bool {
        self.kind() == TokenKind::Dict
    }

    /// Returns true if this node is a string.
    pub fn is_str(&self) -> bool {
        self.kind() == TokenKind::ByteStr
    }

    /// Returns true if this node is a integer.
    pub fn is_int(&self) -> bool {
        self.kind() == TokenKind::Int
    }

    /// Return this node as a `List` which provides further
    /// list operations such as `get`, `iter` etc.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"l1:a2:bce";
    ///     let node = Node::parse(bytes).unwrap();
    ///     let list = node.as_list().unwrap();
    ///     assert_eq!("a", list.get_str(0).unwrap());
    ///     assert_eq!("bc", list.get_str(1).unwrap());
    /// ```
    pub fn as_list(&self) -> Option<List<'_>> {
        if self.is_list() {
            Some(List {
                buf: self.buf,
                tokens: &self.tokens,
                idx: self.idx,
            })
        } else {
            return None;
        }
    }

    /// Return this node as a `Dict` which provides further
    /// dictionary operations such as `get`, `iter` etc.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"d1:a2:bce";
    ///     let node = Node::parse(bytes).unwrap();
    ///     let dict = node.as_dict().unwrap();
    ///     assert_eq!("bc", dict.get_str(b"a").unwrap());
    /// ```
    pub fn as_dict(&self) -> Option<Dict<'_>> {
        if self.is_dict() {
            Some(Dict {
                buf: self.buf,
                tokens: &self.tokens,
                idx: self.idx,
            })
        } else {
            return None;
        }
    }

    /// Return this node as a `i64`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"i123e";
    ///     let node = Node::parse(bytes).unwrap();
    ///     assert_eq!(123, node.as_int().unwrap());
    /// ```
    pub fn as_int(&self) -> Option<i64> {
        let token = &self.tokens[self.idx];
        if token.kind != TokenKind::Int {
            return None;
        }
        let mut val = 0;
        let mut negative = false;
        for &c in &self.buf[token.range()] {
            if c == b'-' {
                negative = true;
            } else {
                let digit = (c - b'0') as i64;
                val = (val * 10) + digit;
            }
        }
        if negative {
            val *= -1
        };
        Some(val)
    }

    /// Return this node as a `i64`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    ///     use ben::Node;
    ///
    ///     let bytes = b"3:abc";
    ///     let node = Node::parse(bytes).unwrap();
    ///     assert_eq!("abc", node.as_str().unwrap());
    /// ```
    pub fn as_str(&self) -> Option<&'a str> {
        let token = &self.tokens[self.idx];
        if let TokenKind::ByteStr = token.kind {
            let bytes = &self.buf[token.range()];
            std::str::from_utf8(bytes).ok()
        } else {
            return None;
        }
    }
}

/// A bencode list
pub struct List<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    idx: usize,
}

impl<'a> List<'a> {
    /// Gets an iterator over the entries of the list
    pub fn iter(&self) -> ListIter<'a> {
        ListIter {
            buf: self.buf,
            tokens: self.tokens,
            total: self.tokens[self.idx].children as usize,
            token_idx: self.idx + 1,
            pos: 0,
        }
    }

    /// Returns the `Node` at the given index.
    pub fn get(&self, i: usize) -> Option<Node<'a>> {
        Some(Node {
            buf: self.buf,
            idx: self.find_idx(i)?,
            tokens: Cow::Borrowed(self.tokens),
        })
    }

    /// Returns the `Dict` at the given index.
    pub fn get_dict(&self, i: usize) -> Option<Dict<'_>> {
        Some(Dict {
            buf: self.buf,
            idx: self.find_idx(i)?,
            tokens: self.tokens,
        })
    }

    /// Returns the `List` at the given index.
    pub fn get_list(&self, i: usize) -> Option<List<'_>> {
        Some(List {
            buf: self.buf,
            idx: self.find_idx(i)?,
            tokens: self.tokens,
        })
    }

    /// Returns the `str` at the given index.
    pub fn get_str(&self, i: usize) -> Option<&str> {
        self.get(i)?.as_str()
    }

    /// Returns the `i64` at the given index.
    pub fn get_int(&self, i: usize) -> Option<i64> {
        self.get(i)?.as_int()
    }

    fn find_idx(&self, i: usize) -> Option<usize> {
        let token = self.tokens.get(self.idx)?;
        if i >= token.children as usize {
            return None;
        }
        let mut idx = self.idx + 1;
        let mut item = 0;

        while item < i {
            idx += self.tokens[idx].next as usize;
            item += 1;
        }

        Some(idx)
    }
}

pub struct ListIter<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    total: usize,
    token_idx: usize,
    pos: usize,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.total {
            return None;
        }

        let idx = self.token_idx;
        self.token_idx += self.tokens[self.token_idx].next as usize;
        self.pos += 1;

        Some(Node {
            buf: self.buf,
            idx,
            tokens: Cow::Borrowed(self.tokens),
        })
    }
}

/// A bencode dictionary
pub struct Dict<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    idx: usize,
}

impl<'a> Dict<'a> {
    /// Gets an iterator over the entries of the dictionary.
    pub fn iter(&self) -> DictIter<'a> {
        DictIter {
            buf: self.buf,
            tokens: self.tokens,
            total: self.tokens[self.idx].children as usize,
            token_idx: self.idx + 1,
            pos: 0,
        }
    }

    /// Returns the `Node` for the given key.
    pub fn get(&self, key: &[u8]) -> Option<Node<'a>> {
        self.iter().find(|(k, _)| k.data() == key).map(|(_, v)| v)
    }

    /// Returns the `Dict` for the given key.
    pub fn get_dict(&self, key: &[u8]) -> Option<Dict<'a>> {
        Some(Dict {
            buf: self.buf,
            idx: self.get(key)?.idx,
            tokens: self.tokens,
        })
    }

    /// Returns the `List` for the given key.
    pub fn get_list(&self, key: &[u8]) -> Option<List<'a>> {
        Some(List {
            buf: self.buf,
            idx: self.get(key)?.idx,
            tokens: self.tokens,
        })
    }

    /// Returns the `str` for the given key.
    pub fn get_str(&self, key: &[u8]) -> Option<&str> {
        self.get(key)?.as_str()
    }

    /// Returns the `i64` for the given key.
    pub fn get_int(&self, key: &[u8]) -> Option<i64> {
        self.get(key)?.as_int()
    }
}

pub struct DictIter<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    total: usize,
    token_idx: usize,
    pos: usize,
}

impl<'a> Iterator for DictIter<'a> {
    type Item = (Node<'a>, Node<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.total {
            return None;
        }

        debug_assert!(self.token_idx < self.tokens.len());
        let key_idx = self.token_idx;

        debug_assert_eq!(TokenKind::ByteStr, self.tokens[key_idx].kind);
        self.token_idx += self.tokens.get(self.token_idx)?.next as usize;

        debug_assert!(self.token_idx < self.tokens.len());
        let val_idx = self.token_idx;
        self.token_idx += self.tokens.get(self.token_idx)?.next as usize;

        self.pos += 2;

        Some((
            Node {
                idx: key_idx,
                buf: self.buf,
                tokens: Cow::Borrowed(self.tokens),
            },
            Node {
                idx: val_idx,
                buf: self.buf,
                tokens: Cow::Borrowed(self.tokens),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::*;

    #[test]
    fn list_get() {
        let s = b"ld1:alee1:be";
        let node = Node::parse(s).unwrap();
        let list = node.as_list().unwrap();
        let n = list.get(1).unwrap();
        assert_eq!(b"b", n.data());
    }

    #[test]
    fn list_get_nested() {
        let s = b"l1:ad1:al1:aee1:be";
        let node = Node::parse(s).unwrap();
        let node = node.as_list().unwrap();
        assert_eq!(b"a", node.get(0).unwrap().data());
        assert_eq!(b"d1:al1:aee", node.get(1).unwrap().data());
        assert_eq!(b"b", node.get(2).unwrap().data());
        assert_eq!(None, node.get(3));
    }

    #[test]
    fn list_get_overflow() {
        let s = b"l1:al1:ad1:al1:aee1:be1:be";
        let node = Node::parse(s).unwrap();
        let node = node.as_list().unwrap();
        let node = node.get_list(1).unwrap();
        assert_eq!(b"a", node.get(0).unwrap().data());
        assert_eq!(b"d1:al1:aee", node.get(1).unwrap().data());
        assert_eq!(b"b", node.get(2).unwrap().data());
        assert_eq!(None, node.get(3));
    }

    #[test]
    fn list_iter() {
        let s = b"l1:ad1:al1:aee1:be";
        let node = Node::parse(s).unwrap();
        let mut iter = node.as_list().unwrap().iter();
        assert_eq!(b"a", iter.next().unwrap().data());
        assert_eq!(b"d1:al1:aee", iter.next().unwrap().data());
        assert_eq!(b"b", iter.next().unwrap().data());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn list_iter_not_a_list() {
        let s = b"de";
        let node = Node::parse(s).unwrap();
        let node = node.as_list();
        assert!(node.is_none());
    }

    #[test]
    fn dict_iter() {
        let s = b"d1:a2:bc3:def4:ghije";
        let node = Node::parse(s).unwrap();
        let mut iter = node.as_dict().unwrap().iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k.data());
        assert_eq!(b"bc", v.data());

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"def", k.data());
        assert_eq!(b"ghij", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter_2() {
        let s = b"d1:alee";
        let node = Node::parse(s).unwrap();
        let mut iter = node.as_dict().unwrap().iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k.data());
        assert_eq!(b"le", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter_inside_list() {
        let s = b"ld1:alee1:a1:ae";
        let node = Node::parse(s).unwrap();
        let mut list_iter = node.as_list().unwrap().iter();

        let dict = list_iter.next().unwrap();
        assert_eq!(b"a", list_iter.next().unwrap().data());
        assert_eq!(b"a", list_iter.next().unwrap().data());
        assert_eq!(None, list_iter.next());

        let mut iter = dict.as_dict().unwrap().iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k.data());
        assert_eq!(b"le", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn int_value() {
        let s = b"i12e";
        let node = Node::parse(s).unwrap();
        assert_eq!(12, node.as_int().unwrap());
    }

    #[test]
    fn int_value_negative() {
        let s = b"i-12e";
        let node = Node::parse(s).unwrap();
        assert_eq!(-12, node.as_int().unwrap());
    }

    #[test]
    fn int_value_invalid() {
        let s = b"ixyze";
        let err = Parser::new().parse(s).unwrap_err();
        assert_eq!(Error::Unexpected { pos: 1 }, err);
    }

    #[test]
    fn str_value() {
        let s = b"5:abcde";
        let node = Node::parse(s).unwrap();
        assert_eq!("abcde", node.as_str().unwrap());
    }

    #[test]
    fn dict_get() {
        let s = b"d1:ai1e1:bi2ee";
        let node = Node::parse(s).unwrap();
        let dict = node.as_dict().unwrap();
        let b = dict.get(b"b").unwrap();
        assert_eq!(2, b.as_int().unwrap());
    }
}
