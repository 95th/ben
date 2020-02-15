use std::collections::BTreeMap;
use std::io::{self, Write};

/// Encoder to use for encoding data into bencode format.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Encoder {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<Self>),
    Dict(BTreeMap<&'static str, Self>),
}

impl Encoder {
    /// Encodes data into a vector of bencoded bytes.
    ///
    /// # Examples:
    /// Basic usage:
    ///
    /// ```
    /// use ben::Encoder;
    ///
    /// let enc = Encoder::from(vec![Encoder::from("Hello"), Encoder::from("World")]);
    /// let bytes = enc.to_vec();
    /// assert_eq!(b"l5:Hello5:Worlde", &bytes[..]);
    /// ```
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = vec![];
        self.write(&mut v).unwrap();
        v
    }

    /// Encodes and writes the data into given `Write` object.
    ///
    /// # Examples:
    /// Basic usage:
    ///
    /// ```
    /// use ben::Encoder;
    ///
    /// let mut bytes = vec![];
    /// let enc = Encoder::from(vec![Encoder::from("Hello"), Encoder::from("World")]);
    /// enc.write(&mut bytes).unwrap();
    /// assert_eq!(b"l5:Hello5:Worlde", &bytes[..]);
    /// ```
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        enum Token<'a> {
            B(&'a Encoder),
            S(&'a str),
            E,
        }

        use Token::*;
        let mut stack = vec![B(self)];
        while !stack.is_empty() {
            match stack.pop().unwrap() {
                Token::B(v) => match v {
                    Self::Int(n) => {
                        write!(w, "i{}e", n)?;
                    }
                    Self::Bytes(v) => {
                        write!(w, "{}:", v.len())?;
                        w.write_all(v)?;
                    }
                    Self::List(v) => {
                        write!(w, "l")?;
                        stack.push(E);
                        stack.extend(v.iter().rev().map(|e| B(e)));
                    }
                    Self::Dict(m) => {
                        write!(w, "d")?;
                        stack.push(E);
                        for (k, v) in m.iter().rev() {
                            stack.push(B(v));
                            stack.push(S(k));
                        }
                    }
                },
                Token::S(s) => {
                    write!(w, "{}:{}", s.len(), s)?;
                }
                Token::E => write!(w, "e")?,
            }
        }
        Ok(())
    }
}

impl From<i64> for Encoder {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<Vec<u8>> for Encoder {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(v)
    }
}

impl From<&[u8]> for Encoder {
    fn from(v: &[u8]) -> Self {
        v.to_vec().into()
    }
}

impl From<&str> for Encoder {
    fn from(v: &str) -> Self {
        v.as_bytes().into()
    }
}

impl From<String> for Encoder {
    fn from(v: String) -> Self {
        v.into_bytes().into()
    }
}

impl From<Vec<Self>> for Encoder {
    fn from(v: Vec<Self>) -> Self {
        Self::List(v)
    }
}

impl From<BTreeMap<&'static str, Self>> for Encoder {
    fn from(v: BTreeMap<&'static str, Self>) -> Self {
        Self::Dict(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_int() {
        let b = Encoder::from(10).to_vec();
        assert_eq!(b"i10e", &b[..]);
    }

    #[test]
    fn encode_str() {
        let b = Encoder::from("1000").to_vec();
        assert_eq!(b"4:1000", &b[..]);
    }

    #[test]
    fn encode_dict() {
        let mut dict = BTreeMap::new();
        dict.insert("Hello", "World".into());
        let b = Encoder::from(dict).to_vec();
        assert_eq!(b"d5:Hello5:Worlde", &b[..]);
    }

    #[test]
    fn encode_list() {
        let mut list: Vec<Encoder> = vec![];
        list.push("Hello".into());
        list.push("World".into());
        list.push(123.into());
        let b = Encoder::from(list).to_vec();
        assert_eq!(b"l5:Hello5:Worldi123ee", &b[..]);
    }
}