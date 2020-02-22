use ben::Node;

fn main() {
    let s = b"d1:a1:b1:ci1e1:x1:y1:dde1:fle1:g1:he";
    let node = Node::parse(s).unwrap();
    println!("{:#?}", node);
}
