use crate::{Token, TokenKind};
use core::fmt;

#[derive(PartialEq)]
pub struct Node<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    idx: usize,
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node").field("idx", &self.idx).finish()
    }
}

impl<'a> Node<'a> {
    pub fn new(buf: &'a [u8], tokens: &'a [Token], idx: usize) -> Self {
        Self { buf, tokens, idx }
    }

    pub fn data(&self) -> &'a [u8] {
        &self.buf[self.tokens[self.idx].range()]
    }

    pub fn kind(&self) -> TokenKind {
        self.tokens[self.idx].kind
    }

    pub fn list_at(&self, i: usize) -> Option<Node<'a>> {
        let token = self.tokens.get(self.idx)?;
        if token.kind != TokenKind::List {
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

        Some(Node { idx, ..*self })
    }

    pub fn list_iter(&self) -> ListIter<'_> {
        let token = &self.tokens[self.idx];
        let pos = match token.kind {
            TokenKind::List => 0,
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
            TokenKind::Dict => 0,
            _ => token.children as usize,
        };
        DictIter {
            node: self,
            total: token.children as usize,
            token_idx: self.idx + 1,
            pos,
        }
    }

    pub fn int_value(&self) -> i64 {
        let token = &self.tokens[self.idx];
        if token.kind != TokenKind::Int {
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

    pub fn str_value(&self) -> &str {
        let token = &self.tokens[self.idx];
        if token.kind != TokenKind::ByteStr {
            return "";
        }
        let bytes = &self.buf[token.range()];
        core::str::from_utf8(bytes).unwrap_or_default()
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

        Some(Node { idx, ..*self.node })
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
        self.token_idx += self.node.tokens.get(self.token_idx)?.next as usize;

        debug_assert!(self.token_idx < self.node.tokens.len());
        let val_idx = self.token_idx;
        self.token_idx += self.node.tokens.get(self.token_idx)?.next as usize;

        self.pos += 2;

        Some((
            Node {
                idx: key_idx,
                ..*self.node
            }
            .data(),
            Node {
                idx: val_idx,
                ..*self.node
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn list_at() {
        let s = b"ld1:alee1:be";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        let n = node.list_at(1).unwrap();
        assert_eq!(b"b", n.data());
    }

    #[test]
    fn list_at_nested() {
        let s = b"l1:ad1:al1:aee1:be";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        assert_eq!(b"a", node.list_at(0).unwrap().data());
        assert_eq!(b"1:al1:ae", node.list_at(1).unwrap().data());
        assert_eq!(b"b", node.list_at(2).unwrap().data());
        assert_eq!(None, node.list_at(3));
    }

    #[test]
    fn list_at_overflow() {
        let s = b"l1:al1:ad1:al1:aee1:be1:be";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 2);
        assert_eq!(b"a", node.list_at(0).unwrap().data());
        assert_eq!(b"1:al1:ae", node.list_at(1).unwrap().data());
        assert_eq!(b"b", node.list_at(2).unwrap().data());
        assert_eq!(None, node.list_at(3));
    }

    #[test]
    fn list_iter() {
        let s = b"l1:ad1:al1:aee1:be";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        let mut iter = node.list_iter();
        assert_eq!(b"a", iter.next().unwrap().data());
        assert_eq!(b"1:al1:ae", iter.next().unwrap().data());
        assert_eq!(b"b", iter.next().unwrap().data());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn list_iter_not_a_list() {
        let s = b"de";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        let mut iter = node.list_iter();
        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter() {
        let s = b"d1:a2:bc3:def4:ghije";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
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
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        let mut iter = node.dict_iter();

        let (k, v) = iter.next().unwrap();
        assert_eq!(b"a", k);
        assert_eq!(b"", v.data());

        assert_eq!(None, iter.next());
    }

    #[test]
    fn dict_iter_inside_list() {
        let s = b"ld1:alee1:a1:ae";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
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
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        assert_eq!(12, node.int_value());
    }

    #[test]
    fn int_value_negative() {
        let s = b"i-12e";
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
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
        let tokens = BenDecoder::new().parse(s).unwrap();
        let node = Node::new(s, &tokens, 0);
        assert_eq!("abcde", node.str_value());
    }
}
