use nexa_scheduler::{LocalScheduler, Scheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn test_scheduler_yielding() {
    let scheduler = LocalScheduler::new();

    // Initially idle
    assert!(scheduler.is_idle());
    assert!(!scheduler.tick());

    // Schedule a task
    scheduler.schedule_microtask(Box::new(|| {}));

    // Not idle
    assert!(!scheduler.is_idle());

    // Tick runs it. Since it drains the queue, it should become idle.
    // So tick() returns false (no more work pending).
    assert!(!scheduler.tick());

    assert!(scheduler.is_idle());
}

#[test]
fn test_cooperative_multitasking() {
    // Two "processes" ping-ponging using microtasks
    let scheduler = Rc::new(LocalScheduler::new());
    let counter = Rc::new(Cell::new(0));

    let s1 = scheduler.clone();
    let c1 = counter.clone();

    // Recursive task formulation requires care with closures/Rc
    // Simulating a simple chain for now.

    s1.schedule_microtask(Box::new(move || {
        c1.set(c1.get() + 1);
    }));

    assert!(!scheduler.tick()); // executed, idle now
    assert_eq!(counter.get(), 1);
    assert!(!scheduler.tick());
}
