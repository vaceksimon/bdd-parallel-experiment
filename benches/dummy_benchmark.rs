use bdd_parallel_experiment::test_utils::load_util::parse_file_path;
use bdd_parallel_experiment::test_utils::parallel::helpers::merge_load;
use criterion::{Criterion, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let file_a = parse_file_path(999);
    let file_b = parse_file_path(994);

    let (mut bdd_actual, a_root, b_root) = merge_load(&file_a, &file_b);

    c.bench_function("bdd parallel 999, 994", |b| {
        b.iter(|| bdd_actual.apply(a_root, b_root))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
