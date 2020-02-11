use ben::BenDecoder;

fn main() {
    let s = b"ld1:alee1:be";
    let tokens = BenDecoder::new().parse(s).unwrap();
    assert_eq!(5, tokens.len());
}
