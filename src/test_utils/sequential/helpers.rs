use crate::sequential::{Bdd, Node, NodeId, TaskVisitor};
use crate::test_utils::load_util::load_biodivine_bdd;
use biodivine_lib_bdd::Bdd as BiodivineBdd;
use std::path::Path;

type ApplyFn<V> = fn(&mut Bdd, NodeId, NodeId, &mut V) -> (NodeId, Node);

/// Load a test file and convert it to our `Bdd`, returning the root node id.
fn load_bdd(path: &Path) -> (Bdd, NodeId) {
    let biodivine: BiodivineBdd = load_biodivine_bdd(path);
    let root: NodeId = NodeId(biodivine.size() - 1);
    (biodivine.into(), root)
}

/// Load two test BDDs, merge them, and run `apply` on their roots.
pub(crate) fn merge_load(a_path: &Path, b_path: &Path) -> (Bdd, NodeId, NodeId) {
    let (bdd_a, a_root) = load_bdd(a_path);
    let (bdd_b, b_root) = load_bdd(b_path);

    let mut merged = bdd_a;
    let id_map = merged.merge(&bdd_b);
    let b_root_merged = id_map[&b_root];

    (merged, a_root, b_root_merged)
}

/// Load two test BDDs, merge them, and run `apply` on their roots.
pub(crate) fn merged_apply<V: TaskVisitor>(
    a_path: &Path,
    b_path: &Path,
    visitor: &mut V,
    apply: ApplyFn<V>,
) -> (Bdd, NodeId, Node) {
    let (mut bdd, a_root, b_root) = merge_load(a_path, b_path);
    let (result_root, result_node) = apply(&mut bdd, a_root, b_root, visitor);
    (bdd, result_root, result_node)
}

/// Extract the subgraph at `root` and convert it to `biodivine-lib-bdd`.
pub(crate) fn extracted_to_biodivine(bdd: &Bdd, root: NodeId) -> BiodivineBdd {
    let (extracted, _) = bdd.extract(root);
    extracted.into()
}

/// Assert that the subgraphs reachable from two roots have identical node lists.
pub(crate) fn assert_same_subgraph(bdd_a: &Bdd, root_a: NodeId, bdd_b: &Bdd, root_b: NodeId) {
    let (extracted_a, _) = bdd_a.extract(root_a);
    let (extracted_b, _) = bdd_b.extract(root_b);

    assert_eq!(
        extracted_a.nodes, extracted_b.nodes,
        "reachable node structures should match"
    );
}
