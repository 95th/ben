use ben::BenDecoder;

fn main() {
    let s = br#"d1:a1:b1:ci1e1:x1:ye"#;
    let p = &mut BenDecoder::new();
    let tokens = &mut [Default::default(); 1000];
    match p.parse(s, tokens) {
        Ok(n) => println!("{:#?}", &tokens[..n]),
        Err(e) => {
            println!("{:#?}", &tokens[..]);
            println!("Error: {:#?}", e);
        }
    }
}
