use ben::{Node, NodeKind};

fn main() {
    let s = b"ld1:alee1:be";
    let node = Node::parse(s).unwrap();
    for item in node.list_iter() {
        if let NodeKind::Dict = item.kind() {
            for (k, v) in item.dict_iter() {
                println!("{:?} => {:?}", k, v);
            }
        } else {
            println!("{:?}", item.data());
        }
    }
}
