pub mod queue;
pub mod scheduler;
pub mod task;

/// The core Scheduler trait that different runtimes can implement.
/// This allows Nexa to run on generic executors (Tokio, Wasm, etc.) or strictly local ones.
pub trait Scheduler {
    /// Schedule a microtask (highest priority, immediate execution).
    /// Used for Promise resolution, signal propagation, etc.
    fn schedule_microtask(&self, task: Box<dyn FnOnce()>);

    /// Schedule a side-effect (runs after microtasks).
    /// Used for DOM updates, logging, etc.
    fn schedule_effect(&self, effect: Box<dyn FnOnce()>);

    /// Schedule a layout effect (runs after standard effects).
    /// Used for measuring layout, reading computed styles.
    fn schedule_layout_effect(&self, effect: Box<dyn FnOnce()>);

    /// Request a cooperative yield to the host system.
    fn request_yield(&self);

    /// Get the current time in milliseconds (monotonic).
    fn now(&self) -> f64;
}

pub use scheduler::LocalScheduler;
