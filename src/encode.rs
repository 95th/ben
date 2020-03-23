use std::io::Write;

pub trait Encoder {
    fn add_int(&mut self, value: i64);

    fn add_byte(&mut self, value: u8);

    fn add_bytes(&mut self, value: &[u8]);

    fn add_str(&mut self, value: &str);

    fn add_list(&mut self) -> List<Self>
    where
        Self: Sized;

    fn add_dict(&mut self) -> Dict<Self>
    where
        Self: Sized;
}

pub struct List<'a, E: Encoder> {
    encoder: &'a mut E,
}

impl<E: Encoder> List<'_, E> {
    pub fn new(encoder: &mut E) -> List<E> {
        encoder.add_byte(b'l');
        List { encoder }
    }

    pub fn add_list(&mut self) -> List<E> {
        List::new(&mut self.encoder)
    }

    pub fn add_dict(&mut self) -> Dict<E> {
        Dict::new(&mut self.encoder)
    }

    pub fn add_str(&mut self, value: &str) {
        self.encoder.add_str(value);
    }

    pub fn add_bytes(&mut self, value: &[u8]) {
        self.encoder.add_bytes(value);
    }

    pub fn add_int(&mut self, value: i64) {
        self.encoder.add_int(value);
    }
}

impl<E: Encoder> Drop for List<'_, E> {
    fn drop(&mut self) {
        self.encoder.add_byte(b'e');
    }
}

pub struct Dict<'a, E: Encoder> {
    encoder: &'a mut E,
}

impl<E: Encoder> Dict<'_, E> {
    pub fn new(encoder: &mut E) -> Dict<E> {
        encoder.add_byte(b'd');
        Dict { encoder }
    }

    pub fn add_list(&mut self, key: &str) -> List<E> {
        self.encoder.add_str(key);
        List::new(&mut self.encoder)
    }

    pub fn add_dict(&mut self, key: &str) -> Dict<E> {
        self.encoder.add_str(key);
        Dict::new(&mut self.encoder)
    }

    pub fn add_str(&mut self, key: &str, value: &str) {
        self.encoder.add_str(key);
        self.encoder.add_str(value);
    }

    pub fn add_bytes(&mut self, key: &str, value: &[u8]) {
        self.encoder.add_str(key);
        self.encoder.add_bytes(value);
    }

    pub fn add_int(&mut self, key: &str, value: i64) {
        self.encoder.add_str(key);
        self.encoder.add_int(value);
    }
}

impl<E: Encoder> Drop for Dict<'_, E> {
    fn drop(&mut self) {
        self.encoder.add_byte(b'e');
    }
}

impl Encoder for Vec<u8> {
    fn add_int(&mut self, value: i64) {
        write!(self, "i{}e", value).unwrap();
    }

    fn add_byte(&mut self, value: u8) {
        self.push(value);
    }

    fn add_bytes(&mut self, value: &[u8]) {
        write!(self, "{}:", value.len()).unwrap();
        self.extend(value);
    }

    fn add_str(&mut self, value: &str) {
        self.add_bytes(value.as_bytes());
    }

    fn add_list(&mut self) -> List<Self> {
        List::new(self)
    }

    fn add_dict(&mut self) -> Dict<Self> {
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
