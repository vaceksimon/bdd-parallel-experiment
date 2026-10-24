#[cfg(test)]
mod tests {
    use crate::sequential::Bdd;
    use crate::sequential::NodeId;
    use crate::test_utils::load_util::load_biodivine_bdd;
    use crate::test_utils::sequential::helpers::{
        assert_same_subgraph, extracted_to_biodivine, merged_apply,
    };
    use biodivine_lib_bdd::Bdd as BiodivineBdd;

    /// Only test BDDs with at most this many nodes (including terminals).
    const MAX_INPUT_NODES: usize = 100;

    #[test]
    fn recursive_and_iterative_apply_agree_on_all_bdd_pairs() {
        use crate::sequential::TaskTracker;
        use crate::test_utils::load_util::load_bdd_files_up_to;

        let files = load_bdd_files_up_to(MAX_INPUT_NODES);
        assert!(
            !files.is_empty(),
            "expected at least one .bdd file in test-data with at most {MAX_INPUT_NODES} nodes"
        );

        for i in 0..files.len() {
            for j in i..files.len() {
                let mut visited_rec: TaskTracker = TaskTracker::new();
                let (bdd_rec, root_rec, node_rec) = merged_apply(
                    &files[i],
                    &files[j],
                    &mut visited_rec,
                    Bdd::apply_recursive_with,
                );
                let mut visited_iter: TaskTracker = TaskTracker::new();
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

                let biodivine_a: BiodivineBdd = load_biodivine_bdd(&files[i]);
                let biodivine_b: BiodivineBdd = load_biodivine_bdd(&files[j]);
                let expected: BiodivineBdd = biodivine_a.and(&biodivine_b);
                let actual: BiodivineBdd = extracted_to_biodivine(&bdd_rec, root_rec);

                let expected_bdd: Bdd = expected.into();
                let expected_root: NodeId = NodeId(expected_bdd.nodes.len() - 1);
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
}
