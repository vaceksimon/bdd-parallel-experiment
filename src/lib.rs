use std::collections::HashMap;

mod bdd;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct NodeId(usize);
type Variable = u32;

#[derive(Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
struct Node {
    variable: Variable,
    low_child: NodeId,
    high_child: NodeId,
}

struct Bdd {
    nodes: Vec<Node>,
    // existing
    node_table: HashMap<Node, NodeId>,
    // finished
    task_cache: HashMap<(NodeId, NodeId), NodeId>,
}
