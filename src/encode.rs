use bytes::BufMut;
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

impl Encode for i64 {
    fn encode<E: Encoder>(&self, enc: &mut E) {
        enc.add_int(*self);
    }
}

/// Bencode Encoder trait.
pub trait Encoder: BufMut {
    /// Encode an integer value.
    fn add_int(&mut self, value: i64);

    /// Encode a byte slice.
    fn add_bytes(&mut self, value: &[u8]);

    /// Encode string slice.
    fn add_str(&mut self, value: &str);

    /// Create a new `List` in this `Encoder`.
    fn add_list(&mut self) -> List<'_, Self>
    where
        Self: Sized;

    /// Create a new `Dict` in this `Encoder`.
    fn add_dict(&mut self) -> Dict<'_, Self>
    where
        Self: Sized;
}

impl<T: BufMut> Encoder for T {
    fn add_int(&mut self, value: i64) {
        self.put_u8(b'i');
        let mut buf = Buffer::new();
        self.put_slice(buf.format(value).as_bytes());
        self.put_u8(b'e');
    }

    fn add_bytes(&mut self, value: &[u8]) {
        let mut buf = Buffer::new();
        self.put_slice(buf.format(value.len()).as_bytes());
        self.put_u8(b':');
        self.put_slice(value);
    }

    fn add_str(&mut self, value: &str) {
        self.add_bytes(value.as_bytes());
    }

    fn add_list(&mut self) -> List<'_, T> {
        List::new(self)
    }

    fn add_dict(&mut self) -> Dict<'_, T> {
        Dict::new(self)
    }
}

/// Bencode List representation.
pub struct List<'a, T: BufMut> {
    enc: &'a mut T,
}

impl<T: BufMut> List<'_, T> {
    pub fn new(enc: &mut T) -> List<'_, T> {
        enc.put_u8(b'l');
        List { enc }
    }

    /// `Encode` a value in this list.
    pub fn add<E: Encode>(&mut self, value: E) {
        value.encode(self.enc);
    }

    /// Create a new `List` in this list.
    pub fn add_list(&mut self) -> List<'_, T> {
        self.enc.add_list()
    }

    /// Create a new `Dict` in this list.
    pub fn add_dict(&mut self) -> Dict<'_, T> {
        self.enc.add_dict()
    }

    /// Finish building this list.
    pub fn finish(self) {}
}

impl<T: BufMut> Drop for List<'_, T> {
    fn drop(&mut self) {
        self.enc.put_u8(b'e');
    }
}

/// Bencode Dictionary representation.
pub struct Dict<'a, T: BufMut> {
    enc: &'a mut T,
}

impl<T: BufMut> Dict<'_, T> {
    pub fn new(enc: &mut T) -> Dict<'_, T> {
        enc.put_u8(b'd');
        Dict { enc }
    }

    /// Create a new `List` for given key inside this dictionary.
    pub fn add_list(&mut self, key: &str) -> List<'_, T> {
        self.enc.add_str(key);
        self.enc.add_list()
    }

    /// Create a new `Dict` for given key inside this dictionary.
    pub fn add_dict(&mut self, key: &str) -> Dict<'_, T> {
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

impl<T: BufMut> Drop for Dict<'_, T> {
    fn drop(&mut self) {
        self.enc.put_u8(b'e');
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
}
