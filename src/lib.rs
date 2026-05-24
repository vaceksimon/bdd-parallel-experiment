use std::collections::HashMap;

pub mod bdd;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(usize);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable(u32);

#[derive(Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Node {
    variable: Variable,
    low_child: NodeId,
    high_child: NodeId,
}

pub struct Bdd {
    nodes: Vec<Node>,
    // existing
    node_table: HashMap<Node, NodeId>,
    // finished
    task_cache: HashMap<(NodeId, NodeId), NodeId>,
}
