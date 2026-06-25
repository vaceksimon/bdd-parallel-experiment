use crate::{Bdd, Node, NodeId};
use biodivine_lib_bdd::Bdd as BiodivineBdd;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

type ApplyFn = fn(&mut Bdd, NodeId, NodeId) -> (NodeId, Node);

/// Only test BDDs with at most this many nodes (including terminals).
const MAX_INPUT_NODES: usize = 100;

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
    let root = NodeId(biodivine.size() - 1);
    (biodivine.into(), root)
}

/// Load two test BDDs, merge them, and run `apply` on their roots.
fn merged_apply(a_path: &Path, b_path: &Path, apply: ApplyFn) -> (Bdd, NodeId, Node) {
    let (bdd_a, a_root) = load_bdd(a_path);
    let (bdd_b, b_root) = load_bdd(b_path);

    let mut merged = bdd_a;
    let id_map = merged.merge(&bdd_b);
    let b_root_merged = id_map[&b_root];

    let (result_root, result_node) = apply(&mut merged, a_root, b_root_merged);
    (merged, result_root, result_node)
}

/// Extract the subgraph at `root` and convert it to `biodivine-lib-bdd`.
fn extracted_to_biodivine(bdd: &Bdd, root: NodeId) -> BiodivineBdd {
    let (extracted, _) = bdd.extract(root);
    extracted.into()
}

/// Assert that the subgraphs reachable from two roots have identical node lists.
fn assert_same_subgraph(bdd_a: &Bdd, root_a: NodeId, bdd_b: &Bdd, root_b: NodeId) {
    let (extracted_a, _) = bdd_a.extract(root_a);
    let (extracted_b, _) = bdd_b.extract(root_b);

    assert_eq!(
        extracted_a.nodes, extracted_b.nodes,
        "reachable node structures should match"
    );
}

#[test]
fn recursive_and_iterative_apply_agree_on_all_bdd_pairs() {
    let files = load_bdd_files_up_to(MAX_INPUT_NODES);
    assert!(
        !files.is_empty(),
        "expected at least one .bdd file in test-data with at most {MAX_INPUT_NODES} nodes"
    );

    for i in 0..files.len() {
        for j in i..files.len() {
            let (bdd_rec, root_rec, node_rec) =
                merged_apply(&files[i], &files[j], Bdd::apply_recursive);
            let (bdd_iter, root_iter, node_iter) =
                merged_apply(&files[i], &files[j], Bdd::apply_iterative);

            assert_eq!(
                node_rec,
                node_iter,
                "root nodes differ for {} and {}",
                files[i].display(),
                files[j].display()
            );
            assert_same_subgraph(&bdd_rec, root_rec, &bdd_iter, root_iter);

            let biodivine_a = load_biodivine_bdd(&files[i]);
            let biodivine_b = load_biodivine_bdd(&files[j]);
            let expected = biodivine_a.and(&biodivine_b);
            let actual = extracted_to_biodivine(&bdd_rec, root_rec);

            let expected_bdd: Bdd = expected.into();
            let expected_root = NodeId(expected_bdd.nodes.len() - 1);
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
