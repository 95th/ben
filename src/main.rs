use ben::*;

fn main() {
    let s = br#"le"#;
    let p = &mut BenDecoder::new();
    let tokens = &mut [Token::default(); 2];
    let n = p.parse(s, tokens).unwrap();
    println!("{:?}", &tokens[..n]);
}
