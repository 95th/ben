use itoa::Buffer;

/// A trait for objects that can be bencoded.
///
/// Types implementing `Encode` are able to be encoded with an instance of
/// `Encoder`.
pub trait Encode {
    /// Feed this value into given `Encoder`.
    fn encode<E: Encoder>(&self, enc: &mut E);

    /// Encode this value into a vector of bytes.
    fn encode_to_vec(&self) -> Vec<u8> {
        let mut encoder = vec![];
        self.encode(&mut encoder);
        encoder
    }
}

impl<T: Encode> Encode for &T {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        (&**self).encode(enc);
    }
}

impl<T: Encode> Encode for Box<T> {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        (&**self).encode(enc);
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        let mut list = enc.add_list();
        for t in self {
            list.add(t);
        }
        list.finish();
    }
}

impl<T: Encode> Encode for [T] {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        let mut list = enc.add_list();
        for t in self {
            list.add(t);
        }
        list.finish();
    }
}

impl Encode for &[u8] {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        enc.add_bytes(self);
    }
}

impl Encode for &str {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        enc.add_str(self);
    }
}

impl Encode for String {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        enc.add_str(self);
    }
}

impl Encode for i64 {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        enc.add_int(*self);
    }
}

macro_rules! impl_arr {
    ( $($len: expr),+ ) => {
        $(
            impl Encode for [u8; $len] {
                fn encode<E: Encoder>(&self, enc: &mut E) {
                    enc.add_bytes(&self[..]);
                }
            }
        )+
    };
}

impl_arr![
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 64, 128, 256, 512, 1024
];

/// Add bytes lazily to given encoder.
///
/// # Panic
/// Drop will panic if the expected number of bytes
/// is not equal to actually added bytes.
pub struct AddBytes<'a> {
    enc: &'a mut Vec<u8>,
    len: usize,
    written: usize,
}

impl AddBytes<'_> {
    /// Add given byte slice.
    pub fn add(&mut self, buf: &[u8]) {
        self.written += buf.len();
        self.enc.extend(buf);
    }

    pub fn finish(self) {}
}

impl Drop for AddBytes<'_> {
    fn drop(&mut self) {
        assert_eq!(self.len, self.written)
    }
}

/// Bencode Encoder trait.
pub trait Encoder {
    /// Encode an integer value.
    fn add_int(&mut self, value: i64);

    /// Encode a byte slice.
    fn add_bytes(&mut self, value: &[u8]);

    /// Create a new object which accepts exactly 'n' bytes lazily.
    ///
    /// The returned object will panic if the total number of added bytes
    /// is not equal to 'n'.
    fn add_n_bytes(&mut self, len: usize) -> AddBytes<'_>;

    /// Encode string slice.
    fn add_str(&mut self, value: &str);

    /// Create a new `List` in this `Encoder`.
    fn add_list(&mut self) -> List<'_>;

    /// Create a new `Dict` in this `Encoder`.
    fn add_dict(&mut self) -> Dict<'_>;
}

impl Encoder for Vec<u8> {
    fn add_int(&mut self, value: i64) {
        self.push(b'i');
        let mut buf = Buffer::new();
        self.extend(buf.format(value).as_bytes());
        self.push(b'e');
    }

    fn add_bytes(&mut self, value: &[u8]) {
        let mut buf = Buffer::new();
        self.extend(buf.format(value.len()).as_bytes());
        self.push(b':');
        self.extend(value);
    }

    fn add_n_bytes(&mut self, len: usize) -> AddBytes<'_> {
        let mut buf = Buffer::new();
        self.extend(buf.format(len).as_bytes());
        self.push(b':');
        AddBytes {
            enc: self,
            len,
            written: 0,
        }
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

/// Bencode List representation.
pub struct List<'a> {
    enc: &'a mut Vec<u8>,
}

impl List<'_> {
    /// create a new list
    pub fn new(enc: &mut Vec<u8>) -> List<'_> {
        enc.push(b'l');
        List { enc }
    }

    /// `Encode` a value in this list.
    pub fn add<E: Encode>(&mut self, value: E) {
        value.encode(self.enc);
    }

    /// Create a new `List` in this list.
    pub fn add_list(&mut self) -> List<'_> {
        self.enc.add_list()
    }

    /// Create a new `Dict` in this list.
    pub fn add_dict(&mut self) -> Dict<'_> {
        self.enc.add_dict()
    }

    /// Finish building this list.
    pub fn finish(self) {}
}

impl Drop for List<'_> {
    fn drop(&mut self) {
        self.enc.push(b'e');
    }
}

/// Bencode Dictionary representation.
///
/// Note: This will not enforce order or uniqueness of keys.
/// These invariants have to be maintained by the caller.
pub struct Dict<'a> {
    enc: &'a mut Vec<u8>,
}

impl Dict<'_> {
    /// Create a new dict
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

    /// `Encode` the value for given key inside this dictionary.
    pub fn add<E: Encode>(&mut self, key: &str, value: E) {
        self.enc.add_str(key);
        value.encode(self.enc);
    }

    /// Finish building this dict.
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
        dict.add("Hello", "World");
        dict.finish();
        assert_eq!(b"d5:Hello5:Worlde", &e[..]);
    }

    #[test]
    fn encode_dict_drop() {
        let mut e = vec![];
        let mut dict = e.add_dict();
        dict.add("Hello", "World");
        drop(dict);
        assert_eq!(b"d5:Hello5:Worlde", &e[..]);
    }

    #[test]
    fn encode_list() {
        let mut e = vec![];
        let mut list = e.add_list();
        list.add("Hello");
        list.add("World");
        list.add(123);
        list.finish();
        assert_eq!(b"l5:Hello5:Worldi123ee", &e[..]);
    }

    #[test]
    fn encode_list_drop() {
        let mut e = vec![];
        let mut list = e.add_list();
        list.add("Hello");
        list.add("World");
        list.add(123);
        drop(list);
        assert_eq!(b"l5:Hello5:Worldi123ee", &e[..]);
    }

    #[test]
    fn encode_custom() {
        enum T {
            A(u8, u8),
            B { x: u32, y: &'static str },
        }

        impl Encode for T {
            fn encode<E: Encoder>(&self, encoder: &mut E) {
                match *self {
                    Self::A(a, b) => {
                        let mut dict = encoder.add_dict();
                        dict.add("0", a as i64);
                        dict.add("1", b as i64);
                    }
                    Self::B { x, y } => {
                        let mut dict = encoder.add_dict();
                        dict.add("x", x as i64);
                        dict.add("y", y);
                    }
                }
            }
        }

        let mut e = vec![];
        let mut list = e.add_list();
        list.add(T::A(1, 2));
        list.add(T::B {
            x: 1,
            y: "Hello world",
        });

        drop(list);
        assert_eq!(&b"ld1:0i1e1:1i2eed1:xi1e1:y11:Hello worldee"[..], &e[..]);
    }

    #[test]
    fn encode_add_bytes2_ok() {
        let mut e = vec![];
        let mut bytes = e.add_n_bytes(4);
        bytes.add(&[0; 2]);
        bytes.add(&[0; 2]);
        drop(bytes);
        assert_eq!(&b"4:\x00\x00\x00\x00"[..], &e[..]);
    }

    #[test]
    #[should_panic]
    fn encode_add_bytes2_panic() {
        let mut e = vec![];
        let mut bytes = e.add_n_bytes(4);
        bytes.add(&[0; 100]);
    }
}
