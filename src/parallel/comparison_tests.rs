use crate::parallel::bdd::tests::generated_tasks_into_set;
use crate::parallel::{Bdd, NodeId};
use crate::sequential::TaskTracker;
use crate::sequential::comparison_tests::merge_load as merge_load_sequential;
use crate::test_utils::load_util::{load_bdd_files_up_to, load_biodivine_bdd};
use biodivine_lib_bdd::{Bdd as BiodivineBdd, BddPointer as BiodivineNodeId};
use std::collections::BTreeSet;
use std::path::Path;

/// Only test BDDs with at most this many nodes (including terminals).
const MAX_INPUT_NODES: usize = 50;

/// Walks through the BDD and checks that all referenced nodes actually exist
fn sanity_check(bdd: &Bdd, root: NodeId) {
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
fn merge_load(a_path: &Path, b_path: &Path) -> (Bdd, NodeId, NodeId) {
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
fn extracted_to_biodivine(bdd: &Bdd, root: NodeId) -> BiodivineBdd {
    let (extracted, extracted_root) = bdd.extract(root);
    extracted.into_biodivine_bdd(extracted_root)
}

#[test]
fn compare_to_biodivine() {
    let files = load_bdd_files_up_to(MAX_INPUT_NODES);
    assert!(
        !files.is_empty(),
        "expected at least one .bdd file in test-data with at most {MAX_INPUT_NODES} nodes"
    );

    for i in 0..files.len() {
        for j in i..files.len() {
            let (mut bdd_actual, a_root, b_root) = merge_load(&files[i], &files[j]);
            let (root_actual, _) = bdd_actual.apply(a_root, b_root);
            sanity_check(&bdd_actual, root_actual);

            let biodivine_a: BiodivineBdd = load_biodivine_bdd(&files[i]);
            let biodivine_b: BiodivineBdd = load_biodivine_bdd(&files[j]);
            let expected: BiodivineBdd = biodivine_a.and(&biodivine_b);
            let actual: BiodivineBdd = extracted_to_biodivine(&bdd_actual, root_actual);

            let expected_root: NodeId = expected.root_pointer().into();
            let expected_bdd: Bdd = expected.into();
            sanity_check(&expected_bdd, expected_root);

            let expected_canonical: BiodivineBdd =
                extracted_to_biodivine(&expected_bdd, expected_root);

            assert_eq!(
                actual,
                expected_canonical,
                "apply result differs from biodivine AND for {} and {}",
                files[i].display(),
                files[j].display()
            );
        }
    }
}

#[test]
fn compare_generated_tasks() {
    let files = load_bdd_files_up_to(MAX_INPUT_NODES);
    assert!(
        !files.is_empty(),
        "expected at least one .bdd file in test-data with at most {MAX_INPUT_NODES} nodes"
    );

    for i in 0..files.len() {
        for j in i..files.len() {
            let (mut bdd_parallel, a_root_parallel, b_root_parallel) =
                merge_load(&files[i], &files[j]);
            let task_set: BTreeSet<(NodeId, NodeId)> = generated_tasks_into_set(
                bdd_parallel.generate_tasks(a_root_parallel, b_root_parallel, 5),
            );

            let (mut bdd_sequential, a_root_sequential, b_root_sequential) =
                merge_load_sequential(&files[i], &files[j]);
            let mut visited_iter: TaskTracker = TaskTracker::new();
            bdd_sequential.apply_iterative_with(
                a_root_sequential,
                b_root_sequential,
                &mut visited_iter,
            );
            let expected_results: BTreeSet<(NodeId, NodeId)> = visited_iter
                .into_iter()
                .map(|(a, b)| (a.0.into(), b.0.into()))
                .collect();

            // Ideally, I would check the exact tasks, but the parallel apply generates random node ids - perhaps choose deterministic ID generation strategy
            assert_eq!(
                expected_results.len(),
                task_set.len(),
                "Concurrent and sequential implementation does not agree on number of generated tasks"
            );
        }
    }
}
