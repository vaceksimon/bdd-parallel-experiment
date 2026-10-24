use crate::parallel::{Bdd, GeneratedTask, NodeId, Variable};
use crate::test_utils::load_util::load_biodivine_bdd;
use biodivine_lib_bdd::{Bdd as BiodivineBdd, BddPointer as BiodivineNodeId};
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn generated_tasks_into_set(
    generated_tasks: DashMap<Variable, SegQueue<GeneratedTask>>,
) -> BTreeSet<(NodeId, NodeId)> {
    generated_tasks
        .iter()
        .flat_map(|value| {
            let queue = value.value();
            let mut new_queue = Vec::new();
            while !queue.is_empty() {
                let (a, b, _, _) = queue.pop().unwrap();
                new_queue.push((a, b));
            }
            new_queue.into_iter()
        })
        .collect()
}

/// Walks through the BDD and checks that all referenced nodes actually exist
pub(crate) fn sanity_check(bdd: &Bdd, root: NodeId) {
    let mut stack = Vec::with_capacity(bdd.nodes.len());
    stack.push(root);

    while let Some(node_id) = stack.pop() {
        let node = *bdd
            .nodes
            .get(&node_id)
            .expect("Sanity check failed")
            .value();
        let high_child = node.high_child;
        if !high_child.is_terminal() {
            stack.push(high_child);
        }
        let low_child = node.low_child;
        if !low_child.is_terminal() {
            stack.push(low_child);
        }
    }
}

/// Load a test file and convert it to our `Bdd`, returning the root node id.
fn load_bdd(path: &Path) -> (Bdd, NodeId) {
    let biodivine: BiodivineBdd = load_biodivine_bdd(path);

    let biodivine_root: BiodivineNodeId = biodivine.root_pointer();
    let bdd: Bdd = biodivine.into();
    let root_id: NodeId = biodivine_root.into();

    (bdd, root_id)
}

/// Load two test BDDs and merge them
pub fn merge_load(a_path: &Path, b_path: &Path) -> (Bdd, NodeId, NodeId) {
    let (bdd_a, a_root) = load_bdd(a_path);
    let (bdd_b, b_root) = load_bdd(b_path);

    let mut merged = bdd_a;
    let id_map = merged.merge(&bdd_b);
    let b_root_merged = id_map[&b_root];

    sanity_check(&merged, a_root);
    sanity_check(&merged, b_root_merged);
    (merged, a_root, b_root_merged)
}

/// Extract the subgraph at `root` and convert it to `biodivine-lib-bdd`.
pub(crate) fn extracted_to_biodivine(bdd: &Bdd, root: NodeId) -> BiodivineBdd {
    let (extracted, extracted_root) = bdd.extract(root);
    extracted.into_biodivine_bdd(extracted_root)
}
