use bdd_parallel_experiment::test_utils::load_util::{parse_big_file_path, parse_file_path};
use bdd_parallel_experiment::test_utils::parallel::helpers::merge_load;
#[allow(unused_imports)]
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Instant;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("BDD");
    group.sample_size(10);

    {
        let file_a_1 = parse_file_path(999);
        let file_b_1 = parse_file_path(994);

        let start_1 = Instant::now();
        let (mut bdd_actual_1, a_root_1, b_root_1) = merge_load(&file_a_1, &file_b_1);
        let end_1 = Instant::now();
        let elapsed_1 = end_1.duration_since(start_1);
        println!("Load time for 999 994: {:?}", elapsed_1);

        group.bench_function("999 994", |b| {
            b.iter(|| bdd_actual_1.apply(a_root_1, b_root_1))
        });
        drop(bdd_actual_1);
    }

    {
        let file_a_2 = parse_big_file_path(3789);
        let file_b_2 = parse_big_file_path(2789);

        let start_2 = Instant::now();
        let (mut bdd_actual_2, a_root_2, b_root_2) = merge_load(&file_a_2, &file_b_2);
        let end_2 = Instant::now();
        let elapsed_2 = end_2.duration_since(start_2);
        println!("Load time for 3789 2789: {:?}", elapsed_2);

        group.bench_function("3789 2789", |b| {
            b.iter(|| bdd_actual_2.apply(a_root_2, b_root_2))
        });
        drop(bdd_actual_2);
    }

    {
        let file_a_3 = parse_big_file_path(17539);
        let file_b_3 = parse_big_file_path(14691);

        let start_3 = Instant::now();
        let (mut bdd_actual_3, a_root_3, b_root_3) = merge_load(&file_a_3, &file_b_3);
        let end_3 = Instant::now();
        let elapsed_3 = end_3.duration_since(start_3);
        println!("Load time for 17539 14691: {:?}", elapsed_3);

        group.bench_function("17539 14691", |b| {
            b.iter(|| bdd_actual_3.apply(a_root_3, b_root_3))
        });
        drop(bdd_actual_3);
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
