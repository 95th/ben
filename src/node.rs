use crate::parse::{BenDecoder, Token};
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum NodeKind {
    Dict,
    List,
    ByteStr,
    Int,
}

#[derive(PartialEq)]
pub struct Node<'a> {
    buf: &'a [u8],
    tokens: Cow<'a, [Token]>,
    idx: usize,
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("idx", &self.idx)
            .field("token", &self.tokens[self.idx])
            .finish()
    }
}

impl<'a> Node<'a> {
    pub fn parse(buf: &'a [u8]) -> crate::Result<Self> {
        Self::parse_max_tokens(buf, usize::max_value())
    }

    pub fn parse_in(buf: &'a [u8], tokens: &'a mut Vec<Token>) -> crate::Result<Self> {
        let decoder = BenDecoder::new();
        decoder.parse_in(buf, tokens)?;
        Ok(Self {
            buf,
            tokens: Cow::Borrowed(tokens),
            idx: 0,
        })
    }

    pub fn parse_prefix(buf: &'a [u8]) -> crate::Result<(Self, usize)> {
        let decoder = BenDecoder::new();
        let (tokens, len) = decoder.parse_prefix(buf)?;
        let node = Self {
            buf,
            tokens: Cow::Owned(tokens),
            idx: 0,
        };
        Ok((node, len))
    }

    pub fn parse_prefix_in(
        buf: &'a [u8],
        tokens: &'a mut Vec<Token>,
    ) -> crate::Result<(Self, usize)> {
        let decoder = BenDecoder::new();
        let len = decoder.parse_prefix_in(buf, tokens)?;
        let node = Self {
            buf,
            tokens: Cow::Borrowed(tokens),
            idx: 0,
        };
        Ok((node, len))
    }

    pub fn parse_max_tokens(buf: &'a [u8], max_tokens: usize) -> crate::Result<Self> {
        let mut decoder = BenDecoder::new();
        decoder.set_token_limit(max_tokens);
        Ok(Self {
            buf,
            tokens: Cow::Owned(decoder.parse(buf)?),
            idx: 0,
        })
    }

    pub fn data(&self) -> &'a [u8] {
        &self.buf[self.tokens[self.idx].range()]
    }

    pub fn kind(&self) -> NodeKind {
        self.tokens[self.idx].kind
    }

    pub fn is_list(&self) -> bool {
        self.kind() == NodeKind::List
    }

    pub fn is_dict(&self) -> bool {
        self.kind() == NodeKind::Dict
    }

    pub fn is_str(&self) -> bool {
        self.kind() == NodeKind::ByteStr
    }

    pub fn is_int(&self) -> bool {
        self.kind() == NodeKind::Int
    }

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

    pub fn as_int(&self) -> i64 {
        let token = &self.tokens[self.idx];
        if token.kind != NodeKind::Int {
            return 0;
        }
        let mut val = 0;
        let mut negative = false;
        for &c in &self.buf[token.range()] {
            if c == b'-' {
                negative = true;
                continue;
            }
            val *= 10;
            let digit = (c - b'0') as i64;
            val += digit;
        }
        if negative {
            -val
        } else {
            val
        }
    }

    pub fn as_str(&self) -> &'a str {
        let token = &self.tokens[self.idx];
        if token.kind != NodeKind::ByteStr {
            return "";
        }
        let bytes = &self.buf[token.range()];
        std::str::from_utf8(bytes).unwrap_or_default()
    }
}

pub struct List<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    idx: usize,
}

impl<'a> List<'a> {
    pub fn data(&self) -> &'a [u8] {
        &self.buf[self.tokens[self.idx].range()]
    }

    pub fn iter(&self) -> ListIter<'a> {
        ListIter {
            buf: self.buf,
            tokens: self.tokens,
            total: self.tokens[self.idx].children as usize,
            token_idx: self.idx + 1,
            pos: 0,
        }
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

    pub fn get(&self, i: usize) -> Option<Node<'a>> {
        Some(Node {
            buf: self.buf,
            idx: self.find_idx(i)?,
            tokens: Cow::Borrowed(self.tokens),
        })
    }

    pub fn get_dict(&self, i: usize) -> Option<Dict<'_>> {
        Some(Dict {
            buf: self.buf,
            idx: self.find_idx(i)?,
            tokens: self.tokens,
        })
    }

    pub fn get_list(&self, i: usize) -> Option<List<'_>> {
        Some(List {
            buf: self.buf,
            idx: self.find_idx(i)?,
            tokens: self.tokens,
        })
    }

    pub fn get_str(&self, i: usize) -> &str {
        self.get(i).map(|s| s.as_str()).unwrap_or_default()
    }

    pub fn get_int(&self, i: usize) -> i64 {
        self.get(i).map(|n| n.as_int()).unwrap_or_default()
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

pub struct Dict<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    idx: usize,
}

impl<'a> Dict<'a> {
    pub fn data(&self) -> &'a [u8] {
        &self.buf[self.tokens[self.idx].range()]
    }

    pub fn iter(&self) -> DictIter<'a> {
        DictIter {
            buf: self.buf,
            tokens: self.tokens,
            total: self.tokens[self.idx].children as usize,
            token_idx: self.idx + 1,
            pos: 0,
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<Node<'a>> {
        self.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    pub fn get_dict(&self, key: &[u8]) -> Option<Dict<'a>> {
        Some(Dict {
            buf: self.buf,
            idx: self.get(key)?.idx,
            tokens: self.tokens,
        })
    }

    pub fn get_list(&self, key: &[u8]) -> Option<List<'a>> {
        Some(List {
            buf: self.buf,
            idx: self.get(key)?.idx,
            tokens: self.tokens,
        })
    }

    pub fn get_str(&self, key: &[u8]) -> &str {
        self.get(key).map(|s| s.as_str()).unwrap_or_default()
    }

    pub fn get_int(&self, key: &[u8]) -> i64 {
        self.get(key).map(|n| n.as_int()).unwrap_or_default()
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
    type Item = (&'a [u8], Node<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.total {
            return None;
        }

        debug_assert!(self.token_idx < self.tokens.len());
        let key_idx = self.token_idx;

        debug_assert_eq!(NodeKind::ByteStr, self.tokens[key_idx].kind);
        self.token_idx += self.tokens.get(self.token_idx)?.next as usize;

        debug_assert!(self.token_idx < self.tokens.len());
        let val_idx = self.token_idx;
        self.token_idx += self.tokens.get(self.token_idx)?.next as usize;

        self.pos += 2;

        let key_range = self.tokens[key_idx].range();
        let key = &self.buf[key_range];

        Some((
            key,
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
        assert_eq!(b"1:al1:ae", node.get(1).unwrap().data());
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
        assert_eq!(b"1:al1:ae", node.get(1).unwrap().data());
        assert_eq!(b"b", node.get(2).unwrap().data());
        assert_eq!(None, node.get(3));
    }

    #[test]
    fn list_iter() {
        let s = b"l1:ad1:al1:aee1:be";
        let node = Node::parse(s).unwrap();
        let mut iter = node.as_list().unwrap().iter();
        assert_eq!(b"a", iter.next().unwrap().data());
        assert_eq!(b"1:al1:ae", iter.next().unwrap().data());
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
        assert_eq!(b"a", k);
        assert_eq!(b"bc", v.data());

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"def", k);
        assert_eq!(b"ghij", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter_2() {
        let s = b"d1:alee";
        let node = Node::parse(s).unwrap();
        let mut iter = node.as_dict().unwrap().iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k);
        assert_eq!(b"", v.data());

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
        assert_eq!(b"a", k);
        assert_eq!(b"", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn int_value() {
        let s = b"i12e";
        let node = Node::parse(s).unwrap();
        assert_eq!(12, node.as_int());
    }

    #[test]
    fn int_value_negative() {
        let s = b"i-12e";
        let node = Node::parse(s).unwrap();
        assert_eq!(-12, node.as_int());
    }

    #[test]
    fn int_value_invalid() {
        let s = b"ixyze";
        let err = BenDecoder::new().parse(s).unwrap_err();
        assert_eq!(Error::Invalid, err);
    }

    #[test]
    fn str_value() {
        let s = b"5:abcde";
        let node = Node::parse(s).unwrap();
        assert_eq!("abcde", node.as_str());
    }

    #[test]
    fn dict_get() {
        let s = b"d1:ai1e1:bi2ee";
        let node = Node::parse(s).unwrap();
        let dict = node.as_dict().unwrap();
        let b = dict.get(b"b").unwrap();
        assert_eq!(2, b.as_int());
    }
}
