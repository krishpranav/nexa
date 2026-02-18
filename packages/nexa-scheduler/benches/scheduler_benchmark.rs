use criterion::{Criterion, black_box, criterion_group, criterion_main};
use nexa_scheduler::{LocalScheduler, Scheduler};

fn benchmark_microtasks(c: &mut Criterion) {
    c.bench_function("schedule_microtask 1000", |b| {
        b.iter(|| {
            let scheduler = LocalScheduler::new();
            for _ in 0..1000 {
                scheduler.schedule_microtask(Box::new(|| {
                    black_box(1 + 1);
                }));
            }
            scheduler.tick();
        })
    });
}

fn benchmark_effects(c: &mut Criterion) {
    c.bench_function("schedule_effect 1000", |b| {
        b.iter(|| {
            let scheduler = LocalScheduler::new();
            for _ in 0..1000 {
                scheduler.schedule_effect(Box::new(|| {
                    black_box(1 + 1);
                }));
            }
            scheduler.tick();
        })
    });
}

criterion_group!(benches, benchmark_microtasks, benchmark_effects);
criterion_main!(benches);
