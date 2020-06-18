use std::fmt;
use std::ops::Range;

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

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    Dict,
    List,
    ByteStr,
    Int,
}
