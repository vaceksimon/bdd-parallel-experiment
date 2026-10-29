use bdd_parallel_experiment::test_utils::load_util::parse_big_file_path;
use bdd_parallel_experiment::test_utils::parallel::helpers::merge_load;
use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Instant;

fn criterion_benchmark(c: &mut Criterion) {
    let file_a = parse_big_file_path(17539);
    let file_b = parse_big_file_path(14691);

    let start = Instant::now();
    let (mut bdd_actual, a_root, b_root) = merge_load(&file_a, &file_b);
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    println!("Load time for 17539 14691: {:?}", elapsed);

    c.bench_function("bdd parallel 17539, 14691", |b| {
        b.iter(|| bdd_actual.apply(a_root, b_root))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
