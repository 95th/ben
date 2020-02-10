use crate::{Token, TokenKind};
use core::fmt;

#[derive(PartialEq)]
pub struct Node<'a> {
    buf: &'a [u8],
    tokens: &'a [Token],
    pub idx: usize,
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

    pub fn list_at(&self, i: usize) -> Option<Node<'a>> {
        let this = self.tokens.get(self.idx)?;
        if this.kind != TokenKind::List {
            return None;
        }

        let mut idx = self.idx + 1;
        let mut item = 0;

        while item < i {
            idx += self.tokens.get(idx)?.next;
            item += 1;
        }

        if idx >= self.tokens.len() {
            return None;
        }

        Some(Node { idx, ..*self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

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
    fn list_at() {
        let s = b"ld1:alee1:be";
        let tokens = parse!(s, 5).unwrap();
        let node = Node::new(s, &tokens, 0);
        let n = node.list_at(1).unwrap();
        assert_eq!(b"b", n.data());
    }

    #[test]
    fn list_at_nested() {
        let s = b"l1:ad1:al1:aee1:be";
        let tokens = parse!(s, 7).unwrap();
        let node = Node::new(s, &tokens, 0);
        assert_eq!(b"a", node.list_at(0).unwrap().data());
        assert_eq!(b"1:al1:ae", node.list_at(1).unwrap().data());
        assert_eq!(b"b", node.list_at(2).unwrap().data());
        assert_eq!(None, node.list_at(3));
    }

    // FIXME
    // #[test]
    // fn list_at_overflow() {
    //     let s = b"l1:al1:ad1:al1:aee1:be1:be";
    //     let tokens = parse!(s, 10).unwrap();
    //     let node = Node::new(s, &tokens, 2);
    //     assert_eq!(b"a", node.list_at(0).unwrap().data());
    //     assert_eq!(b"1:al1:ae", node.list_at(1).unwrap().data());
    //     assert_eq!(b"b", node.list_at(2).unwrap().data());
    //     assert_eq!(None, node.list_at(3));
    // }
}
