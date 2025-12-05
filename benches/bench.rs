use criterion::{Criterion, criterion_group, criterion_main};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("exemplo", |b| {
        b.iter(|| {
            (0..1000).sum::<u32>() // código a ser medido
        });
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
