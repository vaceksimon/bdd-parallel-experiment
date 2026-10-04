use dashmap::DashMap;
use uuid::Uuid;

pub mod bdd;
pub mod common;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct NodeId(usize);
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
pub struct SuccessorResults(TaskId, TaskId);

pub struct Bdd {
    nodes: Vec<Node>,
    // existing
    _node_table: DashMap<Node, NodeId>,
    // finished
    _task_cache: DashMap<(NodeId, NodeId), NodeId>,
}

impl From<Uuid> for TaskId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl TaskId {
    pub const TERMINAL: Self = Self(Uuid::nil());
}

impl SuccessorResults {
    pub const TERMINAL: Self = Self(TaskId::TERMINAL, TaskId::TERMINAL);

    pub fn new(a: TaskId, b: TaskId) -> Self {
        Self(a, b)
    }
}

impl From<SuccessorResults> for (TaskId, TaskId) {
    fn from(value: SuccessorResults) -> Self {
        (value.0, value.1)
    }
}
