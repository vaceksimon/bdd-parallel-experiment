use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Condvar, Mutex, RwLock};

pub mod bdd;

/// Implements conversion utilities for `biodivine_lib_bdd`. These enable comparison testing,
/// but don't ship as part of the official API.
#[cfg(test)]
mod biodivine_conversions;

/// These are larger "integration tests" that
#[cfg(test)]
mod comparison_tests;

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

pub struct Bdd {
    nodes: Vec<Node>,
    // existing
    node_table: HashMap<Node, NodeId>,
    // finished
    task_cache: HashMap<(NodeId, NodeId), NodeId>,
}

pub struct BddParallel {
    nodes: RwLock<Vec<Node>>,
    // existing
    node_table: DashMap<Node, NodeId>,
    // finished
    task_cache: DashMap<(NodeId, NodeId), NodeId>,
}

struct WorkerPool {
    stack: Mutex<Vec<(NodeId, NodeId, Variable)>>,
    results: Mutex<Vec<(NodeId, Node)>>,

    // Termination detection state
    is_done: AtomicBool,
    idle_workers: AtomicUsize,

    // Standard library locking primitives used purely for thread sleeping
    sleep_lock: Mutex<()>,
    cvar: Condvar,
}
