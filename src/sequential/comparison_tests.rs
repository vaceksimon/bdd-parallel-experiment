use crate::sequential::{Bdd, NodeVisitor, VisitedNodes};
use crate::sequential::{Node, NodeId};
use crate::test_utils::load_util::{load_bdd_files_up_to, load_biodivine_bdd};
use biodivine_lib_bdd::Bdd as BiodivineBdd;
use std::path::Path;

type ApplyFn<V> = fn(&mut Bdd, NodeId, NodeId, &mut V) -> (NodeId, Node);

/// Only test BDDs with at most this many nodes (including terminals).
const MAX_INPUT_NODES: usize = 100;

/// Load a test file and convert it to our `Bdd`, returning the root node id.
fn load_bdd(path: &Path) -> (Bdd, NodeId) {
    let biodivine = load_biodivine_bdd(path);
    let root = NodeId(biodivine.size() - 1);
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
fn merged_apply<V: NodeVisitor>(
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
            let mut visited_rec: VisitedNodes = VisitedNodes::new();
            let (bdd_rec, root_rec, node_rec) = merged_apply(
                &files[i],
                &files[j],
                &mut visited_rec,
                Bdd::apply_recursive_with,
            );
            let mut visited_iter: VisitedNodes = VisitedNodes::new();
            let (bdd_iter, root_iter, node_iter) = merged_apply(
                &files[i],
                &files[j],
                &mut visited_iter,
                Bdd::apply_iterative_with,
            );

            assert_eq!(
                visited_rec,
                visited_iter,
                "visited nodes differ for {} and {}",
                files[i].display(),
                files[j].display()
            );

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
