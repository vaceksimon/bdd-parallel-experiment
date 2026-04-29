mod bdd;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
struct Node {
    variable: u64,
    low_child: Option<Box<Node>>,
    high_child: Option<Box<Node>>,
}

struct Bdd {
    root: Option<Box<Node>>,
}
