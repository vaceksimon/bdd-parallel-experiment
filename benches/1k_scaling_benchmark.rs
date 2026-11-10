mod common;

use crate::common::{BenchDataset, load_bdds};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    let dataset = [
        BenchDataset {
            bdd_a: 6658,
            bdd_b: 3852,
            result_size: 323,
            cudd_elapsed: 72,
        },
        BenchDataset {
            bdd_a: 2081,
            bdd_b: 1266,
            result_size: 326,
            cudd_elapsed: 108,
        },
        BenchDataset {
            bdd_a: 3306,
            bdd_b: 1683,
            result_size: 3978,
            cudd_elapsed: 735,
        },
        BenchDataset {
            bdd_a: 7386,
            bdd_b: 2093,
            result_size: 8482,
            cudd_elapsed: 1694,
        },
        BenchDataset {
            bdd_a: 6466,
            bdd_b: 3267,
            result_size: 9364,
            cudd_elapsed: 2046,
        },
        // BenchDataset { // TODO could be used elsewhere since this one breaks the scale, i.e. takes too long
        //     bdd_a: 7645,
        //     bdd_b: 3260,
        //     result_size: 17372,
        //     cudd_elapsed: 4110,
        // },
        // BenchDataset { // TODO could be used elsewhere since this one breaks the scale, i.e. takes too long
        //     bdd_a: 7411,
        //     bdd_b: 7050,
        //     result_size: 114894,
        //     cudd_elapsed: 30589,
        // },
    ];

    let mut group = c.benchmark_group("BDDs 1k");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

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
