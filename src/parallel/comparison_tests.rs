use crate::parallel::bdd::tests::{generated_tasks_into_set, make_thesis_example_bdds};
use crate::parallel::{Bdd, Node, NodeId, Variable};
use crate::sequential::VisitedNodes;
use biodivine_lib_bdd::{Bdd as BiodivineBdd, BddNode as BiodivineNode};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

/// Only test BDDs with at most this many nodes (including terminals).
const MAX_INPUT_NODES: usize = 100;

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

/// Path to the `test-data` directory at the crate root.
fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

/// Collect all `.bdd` files from `test-data`, sorted by path.
fn load_bdd_files() -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(test_data_dir())
        .expect("test-data directory should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "bdd"))
        .collect();
    files.sort();
    files
}

/// Parse the node count encoded in a filename like `42.binary.bdd`.
fn bdd_size_from_filename(path: &Path) -> Option<usize> {
    let stem = path.file_stem()?.to_str()?;
    stem.strip_suffix(".binary")?.parse().ok()
}

/// Return test files whose encoded size is at most `max_nodes`.
fn load_bdd_files_up_to(max_nodes: usize) -> Vec<PathBuf> {
    load_bdd_files()
        .into_iter()
        .filter(|path| bdd_size_from_filename(path).is_some_and(|size| size <= max_nodes))
        .collect()
}

/// Load a BDD from a binary-encoded test file using `biodivine-lib-bdd`.
///
/// Normalizes the variable count so that all test BDDs are compatible with each other.
fn load_biodivine_bdd(path: &Path) -> BiodivineBdd {
    let mut file = File::open(path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()));
    let mut bdd = BiodivineBdd::read_as_bytes(&mut file)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    // A small hack to make all BDDs compatible between each other:
    unsafe {
        bdd.set_num_vars(u16::MAX);
    }
    bdd
}

/// Load a test file and convert it to our `Bdd`, returning the root node id.
fn load_bdd(path: &Path) -> (Bdd, NodeId) {
    let biodivine = load_biodivine_bdd(path);

    let root_biodivine = get_biodivine_root(&biodivine);

    let bdd: Bdd = biodivine.into();
    let root = get_root(&bdd, root_biodivine.var.into());

    (bdd, root)
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

fn get_biodivine_root(bdd: &BiodivineBdd) -> BiodivineNode {
    let root_index = bdd.root_pointer().to_index();
    let expected_nodes = bdd.clone().to_nodes();
    expected_nodes[root_index]
}

/// Looks up the root node from the BDD for a given variable.
/// Operates on the fact that the root node is the only node with the given variable.
fn get_root(bdd: &Bdd, root_var: Variable) -> NodeId {
    let root = bdd
        .node_table
        .iter()
        .filter(|pair| pair.key().variable == root_var)
        .map(|pair| (*pair.key(), *pair.value()))
        .collect::<Vec<(Node, NodeId)>>();
    assert_eq!(root.len(), 1);
    root[0].1
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

            let biodivine_a = load_biodivine_bdd(&files[i]);
            let biodivine_b = load_biodivine_bdd(&files[j]);
            let expected: BiodivineBdd = biodivine_a.and(&biodivine_b);
            let expected_root = get_biodivine_root(&expected);
            let actual: BiodivineBdd = extracted_to_biodivine(&bdd_actual, root_actual);

            let expected_bdd: Bdd = expected.into();
            let expected_root = get_root(&expected_bdd, expected_root.var.into());
            sanity_check(&expected_bdd, expected_root);

            let expected_canonical = extracted_to_biodivine(&expected_bdd, expected_root);

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
    let (mut bdd, a_id, b_id) = crate::sequential::bdd::tests::make_thesis_example_bdds();
    let mut visited_iter: VisitedNodes = VisitedNodes::new();
    bdd.apply_iterative_with(a_id, b_id, &mut visited_iter);
    let expected_results: HashSet<(NodeId, NodeId)> = visited_iter
        .into_iter()
        .map(|(a, b)| (a.0.into(), b.0.into()))
        .collect();

    let (mut bdd, a_id, b_id) = make_thesis_example_bdds();

    let task_set: HashSet<(NodeId, NodeId)> =
        generated_tasks_into_set(bdd.generate_tasks(a_id, b_id, 5));

    assert_eq!(expected_results, task_set);
}

#[test]
fn compare_generated_tasks_() {
    let files = load_bdd_files_up_to(MAX_INPUT_NODES);
    assert!(
        !files.is_empty(),
        "expected at least one .bdd file in test-data with at most {MAX_INPUT_NODES} nodes"
    );

    for i in 0..files.len() {
        for j in i..files.len() {
            let (_bdd_actual, _a_root, _b_root) = merge_load(&files[i], &files[j]);
            // TODO compare generated tasks of parallel, iterative and recursive
        }
    }
}
