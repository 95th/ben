use ben::{node::*, *};

fn main() {
    let s = b"ld1:alee1:be";
    let p = &mut BenDecoder::new();
    let tokens = &mut [Default::default(); 1000];
    let n = p.parse(s, tokens).unwrap();
    let node = Node::new(s, &tokens[..n], 0);
    let n = node.list_at(1).unwrap();
    assert_eq!(4, n.idx);
}
