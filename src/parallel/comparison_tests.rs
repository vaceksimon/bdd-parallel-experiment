use crate::parallel::{Bdd, NodeId};
use biodivine_lib_bdd::{Bdd as BiodivineBdd, BddNode as BiodivineNode};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

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

    let (biodivine, root_biodivine) = get_biodivine_root(biodivine);
    let bdd: Bdd = biodivine.into();
    let root = *bdd.node_table.get(&root_biodivine.into()).unwrap().value();

    (bdd, root)
}

/// Load two test BDDs, merge them, and run `apply` on their roots.
fn merged_apply(a_path: &Path, b_path: &Path) -> (Bdd, NodeId) {
    let (bdd_a, a_root) = load_bdd(a_path);
    let (bdd_b, b_root) = load_bdd(b_path);

    let mut merged = bdd_a;
    let id_map = merged.merge(&bdd_b);
    let b_root_merged = id_map[&b_root];

    let (result_root, _) = merged.apply(a_root, b_root_merged);
    (merged, result_root)
}

/// Extract the subgraph at `root` and convert it to `biodivine-lib-bdd`.
fn extracted_to_biodivine(bdd: &Bdd, root: NodeId) -> BiodivineBdd {
    let (extracted, _) = bdd.extract(root);
    extracted.into()
}

fn get_biodivine_root(bdd: BiodivineBdd) -> (BiodivineBdd, BiodivineNode) {
    let root_index = bdd.root_pointer().to_index();
    let expected_nodes = bdd.to_nodes();

    let root = expected_nodes[root_index];
    let reconstructed_bdd = BiodivineBdd::from_nodes(&expected_nodes).unwrap();
    (reconstructed_bdd, root)
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
            let (bdd_actual, root_actual) = merged_apply(&files[i], &files[j]);

            let biodivine_a = load_biodivine_bdd(&files[i]);
            let biodivine_b = load_biodivine_bdd(&files[j]);
            let expected: BiodivineBdd = biodivine_a.and(&biodivine_b);
            // ======
            let (expected, expected_root) = get_biodivine_root(expected);
            // ======
            let actual: BiodivineBdd = extracted_to_biodivine(&bdd_actual, root_actual);

            let expected_bdd: Bdd = expected.into();
            let expected_root = *expected_bdd
                .node_table
                .get(&expected_root.into())
                .unwrap()
                .value();
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
