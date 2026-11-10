mod common;

use crate::common::{BenchDataset, load_bdds};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    let dataset = [
        BenchDataset {
            bdd_a: 3114322,
            bdd_b: 2617673,
            result_size: 1,
            cudd_elapsed: 547015,
        },
        BenchDataset {
            bdd_a: 6290355,
            bdd_b: 1961970,
            result_size: 53370,
            cudd_elapsed: 54140,
        },
        BenchDataset {
            bdd_a: 2617427,
            bdd_b: 1228391,
            result_size: 58966,
            cudd_elapsed: 154170,
        },
        BenchDataset {
            bdd_a: 3855685,
            bdd_b: 1132178,
            result_size: 1,
            cudd_elapsed: 215714,
        },
        // BenchDataset { // TODO could be used elsewhere since this one breaks the scale, i.e. takes too long
        //     bdd_a: 3037325,
        //     bdd_b: 2488493,
        //     result_size: 444041,
        //     cudd_elapsed: 800007,
        // },
    ];

    let mut group = c.benchmark_group("BDDs 1m");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));

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
