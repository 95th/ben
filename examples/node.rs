use ben::Node;

fn main() {
    let s = b"l5:Hello5:Worlde";
    let node = Node::parse(s).unwrap();
    let list = node.as_list().unwrap();
    let list: Vec<_> = list.iter().map(|n| n.as_bytes().unwrap()).collect();
    assert_eq!(&[b"Hello", b"World"], &list[..]);
}
