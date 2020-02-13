use ben::Node;

fn main() {
    let s = b"ld1:alee1:be";
    let tokens = Node::parse(s).unwrap();
    assert!(tokens.is_list());
}
