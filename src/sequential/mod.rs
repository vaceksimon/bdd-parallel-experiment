pub use crate::{Node, NodeId};
use std::collections::HashMap;

pub mod bdd;

/// Implements conversion utilities for `biodivine_lib_bdd`. These enable comparison testing,
/// but don't ship as part of the official API.
#[cfg(test)]
mod biodivine_conversions;
/// These are larger "integration tests" that
#[cfg(test)]
mod comparison_tests;

pub struct Bdd {
    nodes: Vec<Node>,
    // existing
    node_table: HashMap<Node, NodeId>,
    // finished
    task_cache: HashMap<(NodeId, NodeId), NodeId>,
}
