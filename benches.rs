use criterion::{criterion_group, criterion_main, Criterion};

fn bench_test(c: &mut Criterion) {
    c.bench_function("test", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, bench_test);
criterion_main!(benches);
