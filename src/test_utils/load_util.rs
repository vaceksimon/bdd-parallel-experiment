use biodivine_lib_bdd::Bdd as BiodivineBdd;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Path to the `test-data` directory at the crate root.
fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

/// Return test files whose encoded size is at most `max_nodes`.
pub(crate) fn load_bdd_files_up_to(max_nodes: usize) -> Vec<PathBuf> {
    load_bdd_files()
        .into_iter()
        .filter(|path| bdd_size_from_filename(path).is_some_and(|size| size <= max_nodes))
        .collect()
}

/// Parse the node count encoded in a filename like `42.binary.bdd`.
fn bdd_size_from_filename(path: &Path) -> Option<usize> {
    let stem = path.file_stem()?.to_str()?;
    stem.strip_suffix(".binary")?.parse().ok()
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

/// Load a BDD from a binary-encoded test file using `biodivine-lib-bdd`.
///
/// Normalizes the variable count so that all test BDDs are compatible with each other.
pub(crate) fn load_biodivine_bdd(path: &Path) -> BiodivineBdd {
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
