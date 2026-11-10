use bdd_parallel_experiment::parallel::{Bdd, NodeId};
use bdd_parallel_experiment::test_utils::load_util::parse_big_file_path;
use bdd_parallel_experiment::test_utils::parallel::helpers::merge_load;
use std::fmt::{Display, Formatter};
use std::time::Instant;

pub(crate) struct BenchDataset {
    pub(crate) bdd_a: usize,
    pub(crate) bdd_b: usize,
    pub(crate) result_size: usize,
    pub(crate) cudd_elapsed: usize,
}

impl Display for BenchDataset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            r#"==================================
Apply {}
    Resulting size: {} nodes
    CUDD elapsed time: {} µs
==================================
"#,
            self.size_str(),
            self.result_size,
            self.cudd_elapsed
        ))
    }
}

impl BenchDataset {
    pub(crate) fn size_str(&self) -> String {
        format!("{}x{}", self.bdd_a, self.bdd_b)
    }
}
pub(crate) fn load_bdds(ds: &BenchDataset) -> (Bdd, NodeId, NodeId) {
    let file_a = parse_big_file_path(ds.bdd_a);
    let file_b = parse_big_file_path(ds.bdd_b);

    println!("Loading {}...", ds.size_str());

    let start = Instant::now();
    let (bdd_actual, a_root, b_root) = merge_load(&file_a, &file_b);
    let end = Instant::now();
    let elapsed = end.duration_since(start);

    println!("Loading finished in: {:?}\n", elapsed);
    (bdd_actual, a_root, b_root)
}
