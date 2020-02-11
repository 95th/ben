use ben::BenDecoder;

fn main() {
    let s = b"ld1:alee1:be";
    let p = &mut BenDecoder::new();
    let tokens = &mut [Default::default(); 5];
    let n = p.parse(s, tokens).unwrap();
    assert_eq!(5, n);
}
