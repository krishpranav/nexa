use nexa_scheduler::LocalScheduler;
use nexa_signals::dependency::{execute, take_dirty, with_graph};
use nexa_signals::*;
use std::sync::{Arc, Mutex};

fn run_scheduler(scheduler: &mut LocalScheduler) {
    let dirty = take_dirty();
    if !dirty.is_empty() {
        scheduler.schedule(dirty);
    }

    let order = with_graph(|g: &Graph| scheduler.run(g));
    execute(order);
}

#[test]
fn test_signal_propagation_simple() {
    let mut scheduler = Scheduler::new();
    let s = Signal::new(10);
    let c = Computed::new({
        let s = s.clone();
        move || s.get() * 2
    });

    assert_eq!(c.get(), 20);
    s.set(20);
    run_scheduler(&mut scheduler);
    assert_eq!(c.get(), 40);
}

#[test]
fn test_topological_scheduling_diamond() {
    let mut scheduler = Scheduler::new();
    let counter = Arc::new(Mutex::new(0));

    let s = Signal::new(1);

    let a = Computed::new({
        let s = s.clone();
        move || s.get() + 1
    });

    let b = Computed::new({
        let s = s.clone();
        move || s.get() * 2
    });

    let _c = Computed::new({
        let a = a.clone();
        let b = b.clone();
        let counter = counter.clone();
        move || {
            let val = a.get() + b.get();
            *counter.lock().unwrap() += 1;
            val
        }
    });

    // Initial calculation
    assert_eq!(*counter.lock().unwrap(), 1);

    // Update s
    s.set(2);
    run_scheduler(&mut scheduler);

    // 'c' should update exactly once even though it has two paths to 's'
    assert_eq!(*counter.lock().unwrap(), 2);
}
