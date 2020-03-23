use ben::{Entry, Node};

fn main() {
    let a = Entry::from("abc");
    let v = a.to_vec();
    println!("{:?}", v);

    let n = Node::parse(&v).unwrap();
    println!("{:#?}", n);
}
