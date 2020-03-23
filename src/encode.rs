use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// Entry to use for encoding data into bencode format.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Entry(Inner<Self>);

#[derive(Debug, Clone, PartialOrd, PartialEq)]
enum Inner<T> {
    Int(i64),
    Bytes(Cow<'static, [u8]>),
    List(Vec<T>),
    Dict(BTreeMap<&'static str, T>),
}

impl Entry {
    /// Encodes data into a vector of bencoded bytes.
    ///
    /// # Examples:
    /// Basic usage:
    ///
    /// ```
    /// use ben::Entry;
    ///
    /// let enc = Entry::from(vec![Entry::from("Hello"), Entry::from("World")]);
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
    /// use ben::Entry;
    ///
    /// let mut bytes = vec![];
    /// let enc = Entry::from(vec![Entry::from("Hello"), Entry::from("World")]);
    /// enc.write(&mut bytes).unwrap();
    /// assert_eq!(b"l5:Hello5:Worlde", &bytes[..]);
    /// ```
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        enum Token<'a> {
            B(&'a Inner<Entry>),
            S(&'a str),
            E,
        }

        use Inner::*;
        use Token::*;
        let mut stack = vec![B(&self.0)];
        while !stack.is_empty() {
            match stack.pop().unwrap() {
                Token::B(v) => match v {
                    Int(n) => {
                        write!(w, "i{}e", n)?;
                    }
                    Bytes(v) => {
                        write!(w, "{}:", v.len())?;
                        w.write_all(&v[..])?;
                    }
                    List(v) => {
                        write!(w, "l")?;
                        stack.push(E);
                        stack.extend(v.iter().rev().map(|e| B(&e.0)));
                    }
                    Dict(m) => {
                        write!(w, "d")?;
                        stack.push(E);
                        for (k, v) in m.iter().rev() {
                            stack.push(B(&v.0));
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

impl From<i64> for Entry {
    fn from(v: i64) -> Self {
        Self(Inner::Int(v))
    }
}

impl From<Cow<'static, [u8]>> for Entry {
    fn from(v: Cow<'static, [u8]>) -> Self {
        Self(Inner::Bytes(v))
    }
}

impl From<Vec<u8>> for Entry {
    fn from(v: Vec<u8>) -> Self {
        Cow::from(v).into()
    }
}

impl From<&'static [u8]> for Entry {
    fn from(v: &'static [u8]) -> Self {
        Cow::from(v).into()
    }
}

impl From<&'static str> for Entry {
    fn from(v: &'static str) -> Self {
        v.as_bytes().into()
    }
}

impl From<String> for Entry {
    fn from(v: String) -> Self {
        v.into_bytes().into()
    }
}

impl From<Vec<Self>> for Entry {
    fn from(v: Vec<Self>) -> Self {
        Self(Inner::List(v))
    }
}

impl From<BTreeMap<&'static str, Self>> for Entry {
    fn from(v: BTreeMap<&'static str, Self>) -> Self {
        Self(Inner::Dict(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_int() {
        let b = Entry::from(10).to_vec();
        assert_eq!(b"i10e", &b[..]);
    }

    #[test]
    fn encode_str() {
        let b = Entry::from("1000").to_vec();
        assert_eq!(b"4:1000", &b[..]);
    }

    #[test]
    fn encode_dict() {
        let mut dict = BTreeMap::new();
        dict.insert("Hello", "World".into());
        let b = Entry::from(dict).to_vec();
        assert_eq!(b"d5:Hello5:Worlde", &b[..]);
    }

    #[test]
    fn encode_list() {
        let mut list: Vec<Entry> = vec![];
        list.push("Hello".into());
        list.push("World".into());
        list.push(123.into());
        let b = Entry::from(list).to_vec();
        assert_eq!(b"l5:Hello5:Worldi123ee", &b[..]);
    }
}
