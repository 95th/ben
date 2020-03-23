use ben::Encoder;
use ben::Node;

fn main() {
    let mut v = vec![];
    let mut list = v.add_list();
    list.add_int(100);
    list.add_str("hello");

    let mut dict = list.add_dict();
    dict.add_bytes("a", b"b");
    dict.add_str("x", "y");
    drop(dict);

    list.add_int(1);
    drop(list);

    let n = Node::parse(&v).unwrap();
    println!("{:#?}", n);
}
