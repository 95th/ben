use ben::Node;

fn main() {
    let s = b"l5:Hello5:Worlde";
    let node = Node::parse(s).unwrap();
    let list: Vec<_> = node.list_iter().map(|n| n.str_value()).collect();
    assert_eq!(&["Hello", "World"], &list[..]);
}
