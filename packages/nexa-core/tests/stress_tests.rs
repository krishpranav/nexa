use nexa_scheduler::*;
use nexa_signals::context::{propagate, with_graph, with_graph_mut};
use nexa_signals::*;
use std::time::Instant;

fn run_scheduler(scheduler: &mut Scheduler) {
    with_graph_mut(|g: &mut Graph| {
        let dirty = g.take_dirty();
        if !dirty.is_empty() {
            scheduler.schedule(dirty);
        }
    });

    let order = with_graph(|g: &Graph| scheduler.run(g));
    propagate(order);
}

#[test]
fn test_stress_10k_signals() {
    let mut scheduler = Scheduler::new();
    let signals: Vec<_> = (0..10000).map(|i| Signal::new(i)).collect();

    let sum = Computed::new({
        let signals = signals.clone();
        move || {
            let mut s = 0;
            for sig in &signals {
                s += *sig.get();
            }
            s
        }
    });

    assert_eq!(*sum.get(), 49995000);

    let start = Instant::now();
    signals[0].set(10);

    run_scheduler(&mut scheduler);

    let duration = start.elapsed();

    println!("10k signals update + recompute took: {:?}", duration);
    assert_eq!(*sum.get(), 49995010);
}

#[test]
fn test_stress_deep_graph() {
    let mut scheduler = Scheduler::new();
    let root = Signal::new(0);
    let mut current = Computed::new({
        let root = root.clone();
        move || *root.get() + 1
    });

    for _ in 0..500 {
        let prev = current.clone();
        current = Computed::new(move || *prev.get() + 1);
    }

    assert_eq!(*current.get(), 501);

    root.set(10);
    run_scheduler(&mut scheduler);

    assert_eq!(*current.get(), 511);
}
