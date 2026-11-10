mod common;

use crate::common::{BenchDataset, load_bdds};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    let dataset = [
        BenchDataset {
            bdd_a: 150345,
            bdd_b: 138357,
            result_size: 184,
            cudd_elapsed: 1025,
        },
        BenchDataset {
            bdd_a: 460723,
            bdd_b: 115287,
            result_size: 489,
            cudd_elapsed: 2903,
        },
        BenchDataset {
            bdd_a: 204682,
            bdd_b: 153442,
            result_size: 5853,
            cudd_elapsed: 5816,
        },
        BenchDataset {
            bdd_a: 269937,
            bdd_b: 121442,
            result_size: 3,
            cudd_elapsed: 18926,
        },
        BenchDataset {
            bdd_a: 465120,
            bdd_b: 253289,
            result_size: 74549,
            cudd_elapsed: 37783,
        },
    ];

    let mut group = c.benchmark_group("BDDs 100k");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    println!();
    for ds in &dataset {
        let (mut bdd_actual, a_root, b_root) = load_bdds(ds);

        println!("{ds}");
        for workers in 1usize..=8usize {
            group.bench_with_input(
                BenchmarkId::new(ds.size_str(), workers),
                &workers,
                |b, &workers| {
                    b.iter(|| bdd_actual.apply(workers, a_root, b_root));
                },
            );
            println!();
        }
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
