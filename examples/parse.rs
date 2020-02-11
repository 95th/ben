use ben::{BenDecoder, Error};

fn main() {
    let s = b"ld1:alee1:be";
    let p = &mut BenDecoder::new();
    let tokens = &mut [Default::default(); 5];
    let err = p.parse(s, tokens).unwrap_err();
    assert_eq!(Error::Incomplete, err);
}
