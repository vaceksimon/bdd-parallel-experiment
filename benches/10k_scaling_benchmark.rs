mod common;

use crate::common::{BenchDataset, load_bdds};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    let dataset = [
        BenchDataset {
            bdd_a: 58356,
            bdd_b: 37415,
            result_size: 1,
            cudd_elapsed: 1590,
        },
        BenchDataset {
            bdd_a: 82800,
            bdd_b: 13914,
            result_size: 1265,
            cudd_elapsed: 227,
        },
        BenchDataset {
            bdd_a: 89654,
            bdd_b: 12938,
            result_size: 727,
            cudd_elapsed: 4961,
        },
        // BenchDataset { // TODO could be used elsewhere since this one breaks the scale, i.e. is too quick
        //     bdd_a: 17539,
        //     bdd_b: 14691,
        //     result_size: 3693,
        //     cudd_elapsed: 2102,
        // },
        BenchDataset {
            bdd_a: 28831,
            bdd_b: 11506,
            result_size: 35534,
            cudd_elapsed: 8724,
        },
        BenchDataset {
            bdd_a: 24671,
            bdd_b: 15950,
            result_size: 27416,
            cudd_elapsed: 11550,
        },
    ];

    let mut group = c.benchmark_group("BDDs 10k");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(40));

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
