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

impl Encoder for Vec<u8> {
    fn add_int(&mut self, value: i64) {
        self.push(b'i');
        itoa::write(&mut *self, value).unwrap();
        self.push(b'e');
    }

    fn add_bytes(&mut self, value: &[u8]) {
        itoa::write(&mut *self, value.len()).unwrap();
        self.push(b':');
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

/// Bencode List representation
pub struct List<'a> {
    enc: &'a mut Vec<u8>,
}

impl List<'_> {
    pub fn new(enc: &mut Vec<u8>) -> List<'_> {
        enc.push(b'l');
        List { enc }
    }

    /// Create a new `List` in this list
    pub fn add_list(&mut self) -> List<'_> {
        self.enc.add_list()
    }

    /// Create a new `Dict` in this list
    pub fn add_dict(&mut self) -> Dict<'_> {
        self.enc.add_dict()
    }

    /// Encode string slice
    pub fn add_str(&mut self, value: &str) {
        self.enc.add_str(value);
    }

    /// Encode a byte slice.
    pub fn add_bytes(&mut self, value: &[u8]) {
        self.enc.add_bytes(value);
    }

    /// Encode an integer value
    pub fn add_int(&mut self, value: i64) {
        self.enc.add_int(value);
    }

    /// Finish building this list
    pub fn finish(self) {}
}

impl Drop for List<'_> {
    fn drop(&mut self) {
        self.enc.push(b'e');
    }
}

/// Bencode Dictionary representation.
pub struct Dict<'a> {
    enc: &'a mut Vec<u8>,
}

impl Dict<'_> {
    pub fn new(enc: &mut Vec<u8>) -> Dict<'_> {
        enc.push(b'd');
        Dict { enc }
    }

    /// Create a new `List` for given key inside this dictionary.
    pub fn add_list(&mut self, key: &str) -> List<'_> {
        self.enc.add_str(key);
        self.enc.add_list()
    }

    /// Create a new `Dict` for given key inside this dictionary.
    pub fn add_dict(&mut self, key: &str) -> Dict<'_> {
        self.enc.add_str(key);
        self.enc.add_dict()
    }

    /// Encode a new string slice for given key inside this dictionary.
    pub fn add_str(&mut self, key: &str, value: &str) {
        self.enc.add_str(key);
        self.enc.add_str(value);
    }

    /// Encode a new byte slice for given key inside this dictionary.
    pub fn add_bytes(&mut self, key: &str, value: &[u8]) {
        self.enc.add_str(key);
        self.enc.add_bytes(value);
    }

    /// Encode a new integer for given key inside this dictionary.
    pub fn add_int(&mut self, key: &str, value: i64) {
        self.enc.add_str(key);
        self.enc.add_int(value);
    }

    /// Finish building this dict
    pub fn finish(self) {}
}

impl Drop for Dict<'_> {
    fn drop(&mut self) {
        self.enc.push(b'e');
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
        let mut dict = e.add_dict();
        dict.add_str("Hello", "World");
        dict.finish();
        assert_eq!(b"d5:Hello5:Worlde", &e[..]);
    }

    #[test]
    fn encode_dict_drop() {
        let mut e = vec![];
        let mut dict = e.add_dict();
        dict.add_str("Hello", "World");
        drop(dict);
        assert_eq!(b"d5:Hello5:Worlde", &e[..]);
    }

    #[test]
    fn encode_list() {
        let mut e = vec![];
        let mut list = e.add_list();
        list.add_str("Hello");
        list.add_str("World");
        list.add_int(123);
        list.finish();
        assert_eq!(b"l5:Hello5:Worldi123ee", &e[..]);
    }

    #[test]
    fn encode_list_drop() {
        let mut e = vec![];
        let mut list = e.add_list();
        list.add_str("Hello");
        list.add_str("World");
        list.add_int(123);
        drop(list);
        assert_eq!(b"l5:Hello5:Worldi123ee", &e[..]);
    }
}
