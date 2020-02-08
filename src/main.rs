use ben::*;

fn main() {
    let s = br#"d1:a1:be"#;
    let p = &mut BenDecoder::new();
    let tokens = &mut [Token::default(); 3];
    let n = p.parse(s, tokens).unwrap();
    println!("{:?}", &tokens[..n]);
}
