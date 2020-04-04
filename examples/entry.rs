use ben::Encoder;
use ben::Node;

fn main() {
    let mut v = vec![];
    let mut list = v.add_list();
    list.add(100);
    list.add("hello");

    let mut dict = list.add_dict();
    dict.add("a", &b"b"[..]);
    dict.add("x", "y");
    dict.finish();

    list.add(1);
    list.finish();

    let n = Node::parse(&v).unwrap();
    println!("{:#?}", n);
}
