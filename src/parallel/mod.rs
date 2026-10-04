use crate::{Node, NodeId};
use dashmap::DashMap;

pub mod bdd;

pub type Task = (NodeId, NodeId);

pub struct Bdd {
    nodes: Vec<Node>,
    // existing
    _node_table: DashMap<Node, NodeId>,
    // finished
    _task_cache: DashMap<(NodeId, NodeId), NodeId>,
}
