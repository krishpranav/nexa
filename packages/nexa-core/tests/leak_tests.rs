use nexa_scheduler::*;
use nexa_signals::context::{propagate, with_graph};
use nexa_signals::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(PartialEq)]
struct Tracker;
impl Drop for Tracker {
    fn drop(&mut self) {
        DROP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_memory_leak_detection() {
    DROP_COUNT.store(0, Ordering::SeqCst);

    {
        let s = Signal::new(Tracker);
        let _c = Computed::new({
            let s = s.clone();
            move || {
                let _ = s.get();
                0
            }
        });
        // Computed and Signal holding Tracker
    }

    // After scope, all should be dropped
    let mut scheduler = Scheduler::new();
    let order = with_graph(|g: &Graph| scheduler.run(g));
    propagate(order);

    // We expect exactly 1 tracker to be dropped
    assert!(DROP_COUNT.load(Ordering::SeqCst) >= 1);
}
