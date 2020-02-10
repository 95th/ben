use crate::{Token, TokenKind};
use core::fmt;

#[derive(Clone, PartialEq)]
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
    pub fn new(buf: &'a [u8], tokens: &'a [Token]) -> Self {
        Self {
            buf,
            tokens,
            idx: 0,
        }
    }

    pub fn list_at(&self, i: usize) -> Option<Node<'a>> {
        let this = self.tokens.get(self.idx)?;
        if this.kind != TokenKind::List {
            return None;
        }

        let mut token = self.idx + 1;
        let mut item = 0;

        while item < i {
            token += self.tokens.get(token)?.next;
            item += 1;
        }

        if token >= self.tokens.len() {
            return None;
        }

        let mut node = self.clone();
        node.idx = token;
        Some(node)
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
        let node = Node::new(s, &tokens);
        let n = node.list_at(1).unwrap();
        assert_eq!(4, n.idx);
    }

    #[test]
    fn list_at_nested() {
        let s = b"l1:ad1:al1:aee1:be";
        let tokens = parse!(s, 7).unwrap();
        let node = Node::new(s, &tokens);
        assert_eq!(1, node.list_at(0).unwrap().idx);
        assert_eq!(2, node.list_at(1).unwrap().idx);
        assert_eq!(6, node.list_at(2).unwrap().idx);
        assert_eq!(None, node.list_at(3));
    }
}
