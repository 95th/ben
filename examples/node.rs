use ben::{node::*, *};

fn main() {
    let s = b"ld1:alee1:be";
    let p = &mut BenDecoder::new();
    let tokens = &mut [Default::default(); 1000];
    let n = p.parse(s, tokens).unwrap();
    let node = Node::new(s, &tokens[..n], 0);
    for item in node.list_iter() {
        if let TokenKind::Dict = item.kind() {
            for (k, v) in item.dict_iter() {
                println!("{:?} => {:?}", k, v);
            }
        } else {
            println!("{:?}", item.data());
        }
    }
}
