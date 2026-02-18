use futures_task::{ArcWake, waker};
use std::sync::Arc;
use std::task::Waker;

/// A simple task handle that can be woken up.
/// Logic: When wake() is called, it re-schedules the task on the associated scheduler?
/// Actually, wakers are usually for Futures.
/// If we are running a Future, we need to poll it.
/// If check returns Pending, we pass a Waker.
/// When Waker is woken, we execute the future again (poll it).

// For LocalScheduler, we might need a way to wrap a Future into a `FnOnce`.
// But `FnOnce` is one-shot.
// So we need a struct that holds the future and re-submits itself.

// Simplified for now: We won't implement full Future executor logic in `task.rs` yet,
// unless requested. The prompt asked for "Waker integration".
// Let's implement a Waker that calls a callback.

struct SimpleWaker {
    // Thread-safe callback?
    // Waker must be Send + Sync.
    // But LocalScheduler is !Send.
    // We typically use a channel or a thread-safe queue if cross-thread.
    // If single-threaded, we can uses thread_local! or unsafe pointer if we guarantee same thread.
    // But `RawWaker` requirements are strict.

    // For now, let's stub a Waker that assumes single-threaded context or panics/does nothing if wrong thread?
    // Actually, widespread pattern is:
    wake_fn: Box<dyn Fn() + Send + Sync>,
}

impl ArcWake for SimpleWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        (arc_self.wake_fn)();
    }
}

pub fn create_waker(f: impl Fn() + Send + Sync + 'static) -> Waker {
    let simple = SimpleWaker {
        wake_fn: Box::new(f),
    };
    waker(Arc::new(simple))
}
