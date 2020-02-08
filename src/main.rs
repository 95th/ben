use ben::*;

fn main() {
    let s = br#"ld1:ald2:ablleeeeee"#;
    let p = &mut BenDecoder::new();
    let tokens = &mut [Token::default(); 8];
    let n = p.parse(s, tokens).unwrap();
    println!("{:?}", &tokens[..n]);
}
