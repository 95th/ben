use ben::BenDecoder;

fn main() {
    let s = b"ld1:alee1:be";
    let p = &mut BenDecoder::new();
    let tokens = p.parse(s).unwrap();
    assert_eq!(5, tokens.len());
}
