use ben::{Node, NodeKind};

fn main() {
    let s = b"ld1:alee1:be";
    let tokens = Node::parse(s).unwrap();
    assert_eq!(NodeKind::List, tokens.kind());
}
