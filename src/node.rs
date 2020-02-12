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

    pub fn list_at(&self, i: usize) -> Option<Node<'_>> {
        let token = self.tokens.get(self.idx)?;
        if token.kind != NodeKind::List {
            return None;
        }

        if i >= token.children as usize {
            return None;
        }

        let mut idx = self.idx + 1;
        let mut item = 0;

        while item < i {
            idx += self.tokens.get(idx)?.next as usize;
            item += 1;
        }

        Some(Node {
            idx,
            tokens: Cow::Borrowed(&self.tokens),
            ..*self
        })
    }

    pub fn list_iter(&self) -> ListIter<'_> {
        let token = &self.tokens[self.idx];
        let pos = match token.kind {
            NodeKind::List => 0,
            _ => token.children as usize,
        };
        ListIter {
            node: self,
            total: token.children as usize,
            token_idx: self.idx + 1,
            pos,
        }
    }

    pub fn dict_iter(&self) -> DictIter<'_> {
        let token = &self.tokens[self.idx];
        let pos = match token.kind {
            NodeKind::Dict => 0,
            _ => token.children as usize,
        };
        DictIter {
            node: self,
            total: token.children as usize,
            token_idx: self.idx + 1,
            pos,
        }
    }

    pub fn dict_find(&self, key: &[u8]) -> Option<Node<'_>> {
        self.dict_iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    pub fn int_value(&self) -> i64 {
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

    pub fn str_value(&self) -> &'a str {
        let token = &self.tokens[self.idx];
        if token.kind != NodeKind::ByteStr {
            return "";
        }
        let bytes = &self.buf[token.range()];
        std::str::from_utf8(bytes).unwrap_or_default()
    }
}

pub struct ListIter<'a> {
    node: &'a Node<'a>,
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
        self.token_idx += self.node.tokens.get(self.token_idx)?.next as usize;
        self.pos += 1;

        Some(Node {
            idx,
            tokens: Cow::Borrowed(&self.node.tokens),
            ..*self.node
        })
    }
}

pub struct DictIter<'a> {
    node: &'a Node<'a>,
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

        debug_assert!(self.token_idx < self.node.tokens.len());
        let key_idx = self.token_idx;

        debug_assert_eq!(NodeKind::ByteStr, self.node.tokens[key_idx].kind);
        self.token_idx += self.node.tokens.get(self.token_idx)?.next as usize;

        debug_assert!(self.token_idx < self.node.tokens.len());
        let val_idx = self.token_idx;
        self.token_idx += self.node.tokens.get(self.token_idx)?.next as usize;

        self.pos += 2;

        Some((
            Node {
                idx: key_idx,
                tokens: Cow::Borrowed(&self.node.tokens),
                ..*self.node
            }
            .data(),
            Node {
                idx: val_idx,
                tokens: Cow::Borrowed(&self.node.tokens),
                ..*self.node
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::*;

    #[test]
    fn list_at() {
        let s = b"ld1:alee1:be";
        let node = Node::parse(s).unwrap();
        let n = node.list_at(1).unwrap();
        assert_eq!(b"b", n.data());
    }

    #[test]
    fn list_at_nested() {
        let s = b"l1:ad1:al1:aee1:be";
        let node = Node::parse(s).unwrap();
        assert_eq!(b"a", node.list_at(0).unwrap().data());
        assert_eq!(b"1:al1:ae", node.list_at(1).unwrap().data());
        assert_eq!(b"b", node.list_at(2).unwrap().data());
        assert_eq!(None, node.list_at(3));
    }

    #[test]
    fn list_at_overflow() {
        let s = b"l1:al1:ad1:al1:aee1:be1:be";
        let node = Node::parse(s).unwrap();
        let node = node.list_at(1).unwrap();
        assert_eq!(b"a", node.list_at(0).unwrap().data());
        assert_eq!(b"1:al1:ae", node.list_at(1).unwrap().data());
        assert_eq!(b"b", node.list_at(2).unwrap().data());
        assert_eq!(None, node.list_at(3));
    }

    #[test]
    fn list_iter() {
        let s = b"l1:ad1:al1:aee1:be";
        let node = Node::parse(s).unwrap();
        let mut iter = node.list_iter();
        assert_eq!(b"a", iter.next().unwrap().data());
        assert_eq!(b"1:al1:ae", iter.next().unwrap().data());
        assert_eq!(b"b", iter.next().unwrap().data());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn list_iter_not_a_list() {
        let s = b"de";
        let node = Node::parse(s).unwrap();
        let mut iter = node.list_iter();
        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter() {
        let s = b"d1:a2:bc3:def4:ghije";
        let node = Node::parse(s).unwrap();
        let mut iter = node.dict_iter();

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
        let mut iter = node.dict_iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k);
        assert_eq!(b"", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter_inside_list() {
        let s = b"ld1:alee1:a1:ae";
        let node = Node::parse(s).unwrap();
        let mut list_iter = node.list_iter();

        let dict = list_iter.next().unwrap();
        assert_eq!(b"a", list_iter.next().unwrap().data());
        assert_eq!(b"a", list_iter.next().unwrap().data());
        assert_eq!(None, list_iter.next());

        let mut iter = dict.dict_iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k);
        assert_eq!(b"", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn int_value() {
        let s = b"i12e";
        let node = Node::parse(s).unwrap();
        assert_eq!(12, node.int_value());
    }

    #[test]
    fn int_value_negative() {
        let s = b"i-12e";
        let node = Node::parse(s).unwrap();
        assert_eq!(-12, node.int_value());
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
        assert_eq!("abcde", node.str_value());
    }

    #[test]
    fn dict_find() {
        let s = b"d1:ai1e1:bi2ee";
        let node = Node::parse(s).unwrap();
        let b = node.dict_find(b"b").unwrap();
        assert_eq!(2, b.int_value());
    }
}
