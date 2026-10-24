use std::collections::{HashMap, HashSet};

pub mod bdd;

pub mod common;
/// These are larger "integration tests" that
#[cfg(any(test, feature = "test-utils"))]
pub(crate) mod comparison_tests;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct NodeId(pub(crate) usize);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable(pub(crate) u32);

pub trait TaskVisitor {
    fn visit(&mut self, _a: NodeId, _b: NodeId) {}
}

impl TaskVisitor for () {}

pub(crate) type TaskTracker = HashSet<(NodeId, NodeId)>;

#[derive(Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
pub struct Node {
    pub(crate) variable: Variable,
    pub(crate) low_child: NodeId,
    pub(crate) high_child: NodeId,
}

pub struct Bdd {
    pub(crate) nodes: Vec<Node>,
    // existing
    pub(crate) node_table: HashMap<Node, NodeId>,
    // finished
    pub(crate) task_cache: HashMap<(NodeId, NodeId), NodeId>,
}
