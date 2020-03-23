use std::io::Write;

/// Bencode Encoder trait
pub trait Encoder {
    /// Encode an integer value
    fn add_int(&mut self, value: i64);

    /// Encode a byte slice.
    fn add_bytes(&mut self, value: &[u8]);

    /// Encode string slice
    fn add_str(&mut self, value: &str);

    /// Create a new `List` in this `Encoder`.
    fn add_list(&mut self) -> List<'_>;

    /// Create a new `Dict` in this `Encoder`
    fn add_dict(&mut self) -> Dict<'_>;
}

/// Bencode List representation
pub struct List<'a> {
    buf: &'a mut Vec<u8>,
}

impl List<'_> {
    pub fn new(buf: &mut Vec<u8>) -> List<'_> {
        buf.push(b'l');
        List { buf }
    }
}

impl Encoder for List<'_> {
    fn add_list(&mut self) -> List<'_> {
        List::new(self.buf)
    }

    fn add_dict(&mut self) -> Dict<'_> {
        Dict::new(self.buf)
    }

    fn add_str(&mut self, value: &str) {
        self.buf.add_str(value);
    }

    fn add_bytes(&mut self, value: &[u8]) {
        self.buf.add_bytes(value);
    }

    fn add_int(&mut self, value: i64) {
        self.buf.add_int(value);
    }
}

impl Drop for List<'_> {
    fn drop(&mut self) {
        self.buf.push(b'e');
    }
}

/// Bencode Dictionary representation
pub struct Dict<'a> {
    buf: &'a mut Vec<u8>,
}

impl Dict<'_> {
    pub fn new(buf: &mut Vec<u8>) -> Dict<'_> {
        buf.push(b'd');
        Dict { buf }
    }

    /// Create a new `List` for given key inside this dictionary.
    pub fn add_list(&mut self, key: &str) -> List<'_> {
        self.buf.add_str(key);
        List::new(self.buf)
    }

    /// Create a new `Dict` for given key inside this dictionary.
    pub fn add_dict(&mut self, key: &str) -> Dict<'_> {
        self.buf.add_str(key);
        Dict::new(self.buf)
    }

    /// Encode a new string slice for given key inside this dictionary.
    pub fn add_str(&mut self, key: &str, value: &str) {
        self.buf.add_str(key);
        self.buf.add_str(value);
    }

    /// Encode a new byte slice for given key inside this dictionary.
    pub fn add_bytes(&mut self, key: &str, value: &[u8]) {
        self.buf.add_str(key);
        self.buf.add_bytes(value);
    }

    /// Encode a new integer for given key inside this dictionary.
    pub fn add_int(&mut self, key: &str, value: i64) {
        self.buf.add_str(key);
        self.buf.add_int(value);
    }
}

impl Drop for Dict<'_> {
    fn drop(&mut self) {
        self.buf.push(b'e');
    }
}

impl Encoder for Vec<u8> {
    fn add_int(&mut self, value: i64) {
        write!(self, "i{}e", value).unwrap();
    }

    fn add_bytes(&mut self, value: &[u8]) {
        write!(self, "{}:", value.len()).unwrap();
        self.extend(value);
    }

    fn add_str(&mut self, value: &str) {
        self.add_bytes(value.as_bytes());
    }

    fn add_list(&mut self) -> List<'_> {
        List::new(self)
    }

    fn add_dict(&mut self) -> Dict<'_> {
        Dict::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_int() {
        let mut e = vec![];
        e.add_int(10);
        assert_eq!(b"i10e", &e[..]);
    }

    #[test]
    fn encode_str() {
        let mut e = vec![];
        e.add_str("1000");
        assert_eq!(b"4:1000", &e[..]);
    }

    #[test]
    fn encode_dict() {
        let mut e = vec![];
        {
            let mut dict = e.add_dict();
            dict.add_str("Hello", "World");
        }
        assert_eq!(b"d5:Hello5:Worlde", &e[..]);
    }

    #[test]
    fn encode_list() {
        let mut e = vec![];
        {
            let mut list = e.add_list();
            list.add_str("Hello");
            list.add_str("World");
            list.add_int(123);
        }
        assert_eq!(b"l5:Hello5:Worldi123ee", &e[..]);
    }
}
