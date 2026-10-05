use dashmap::DashMap;
use uuid::Uuid;

pub mod bdd;
pub mod common;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct NodeId(Uuid);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable(u32);

#[derive(Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
pub struct Node {
    variable: Variable,
    low_child: NodeId,
    high_child: NodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(Uuid);
pub type Task = (NodeId, NodeId, TaskId);
pub type GeneratedTask = (NodeId, NodeId, TaskId, SuccessorResults);
#[derive(Debug)]
pub struct SuccessorResults(TaskId, TaskId);

pub struct Bdd {
    nodes: DashMap<NodeId, Node>,
    // existing
    node_table: DashMap<Node, NodeId>,
}
