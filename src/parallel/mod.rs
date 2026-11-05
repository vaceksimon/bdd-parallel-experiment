use dashmap::DashMap;
use uuid::Uuid;

pub mod bdd;
pub mod common;
#[cfg(any(test, feature = "test-utils"))]
mod comparison_tests;
pub mod xddd;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct NodeId(pub(crate) Uuid);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable(pub(crate) u32);

#[derive(Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
pub struct Node {
    pub(crate) variable: Variable,
    pub(crate) low_child: NodeId,
    pub(crate) high_child: NodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub(crate) NodeId, pub(crate) NodeId);
pub type Task = (NodeId, NodeId); // TODO identical to above
#[derive(Debug)]
pub struct SuccessorResults(TaskId, TaskId); // TODO 

#[derive(Debug)]
pub struct Bdd {
    pub nodes: DashMap<NodeId, Node>,
    // existing
    pub node_table: DashMap<Node, NodeId>,
}
