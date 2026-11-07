use bdd_parallel_experiment::test_utils::load_util::parse_big_file_path;
use bdd_parallel_experiment::test_utils::parallel::helpers::merge_load;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Instant;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("BDD");
    group.sample_size(10);

    let workers_range = 1usize..=8usize;
    let bdd_a = 3789;
    let bdd_b = 2789;

    let file_a = parse_big_file_path(bdd_a);
    let file_b = parse_big_file_path(bdd_b);

    let start = Instant::now();
    let (mut bdd_actual, a_root, b_root) = merge_load(&file_a, &file_b);
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    println!("Load time for {} {}: {:?}", bdd_a, bdd_b, elapsed);

    for workers in workers_range {
        group.bench_with_input(
            BenchmarkId::new("BDD thousands", workers),
            &workers,
            |b, &workers| {
                b.iter(|| bdd_actual.apply(workers, a_root, b_root));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
