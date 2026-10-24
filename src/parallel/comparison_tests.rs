#[cfg(test)]
mod tests {
    use crate::parallel::{Bdd, NodeId};
    use crate::sequential::TaskTracker;
    use crate::test_utils::load_util::load_bdd_files_up_to;
    use crate::test_utils::load_util::load_biodivine_bdd;
    use crate::test_utils::parallel::helpers::extracted_to_biodivine;
    use crate::test_utils::parallel::helpers::sanity_check;
    use crate::test_utils::parallel::helpers::{generated_tasks_into_set, merge_load};
    use crate::test_utils::sequential::helpers::merge_load as merge_load_sequential;
    use biodivine_lib_bdd::Bdd as BiodivineBdd;
    use std::collections::BTreeSet;

    /// Only test BDDs with at most this many nodes (including terminals).
    const MAX_INPUT_NODES: usize = 50;

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
}
