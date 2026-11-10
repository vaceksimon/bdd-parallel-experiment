use bdd_parallel_experiment::test_utils::load_util::parse_big_file_path;
use bdd_parallel_experiment::test_utils::parallel::helpers::merge_load;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Instant;

struct BenchDataset {
    name: String,
    bdd_a: usize,
    bdd_b: usize,
}

fn criterion_benchmark(c: &mut Criterion) {
    let dataset = [
        BenchDataset {
            name: "1K".to_string(),
            bdd_a: 6658,
            bdd_b: 3852,
        },
        BenchDataset {
            name: "10K".to_string(),
            bdd_a: 17539,
            bdd_b: 14691,
        },
        BenchDataset {
            name: "100K".to_string(),
            bdd_a: 460723,
            bdd_b: 115287,
        },
        BenchDataset {
            name: "1M".to_string(),
            bdd_a: 3114322,
            bdd_b: 2617673,
        },
    ];

    let mut group = c.benchmark_group("BDD");
    group.sample_size(10);

    for ds in &dataset {
        let file_a = parse_big_file_path(ds.bdd_a);
        let file_b = parse_big_file_path(ds.bdd_b);

        let start = Instant::now();
        let (mut bdd_actual, a_root, b_root) = merge_load(&file_a, &file_b);
        let end = Instant::now();
        let elapsed = end.duration_since(start);
        println!("Load time for {} {}: {:?}", ds.bdd_a, ds.bdd_b, elapsed);

        for workers in 1usize..=8usize {
            group.bench_with_input(
                BenchmarkId::new(ds.name.clone(), workers),
                &workers,
                |b, &workers| {
                    b.iter(|| bdd_actual.apply(workers, a_root, b_root));
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
