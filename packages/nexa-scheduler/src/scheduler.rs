use crate::Scheduler;
use crate::queue::TaskQueue;
use std::cell::RefCell;
use std::time::Instant;

/// A single-threaded, cooperative scheduler.
pub struct LocalScheduler {
    microtasks: TaskQueue,
    effects: TaskQueue,
    layout_effects: TaskQueue,
    start_time: Instant,
    // Preventing recursive ticks if needed
    in_tick: RefCell<bool>,
}

impl Default for LocalScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalScheduler {
    pub fn new() -> Self {
        Self {
            microtasks: TaskQueue::new(),
            effects: TaskQueue::new(),
            layout_effects: TaskQueue::new(),
            start_time: Instant::now(),
            in_tick: RefCell::new(false),
        }
    }

    /// Run one cycle of the event loop.
    /// Returns true if there might be more work (queues not empty), false if idle.
    pub fn tick(&self) -> bool {
        if *self.in_tick.borrow() {
            // Re-entrant tick? Avoid busy loop recursion.
            return true;
        }

        *self.in_tick.borrow_mut() = true;

        // 1. Drain Microtasks
        // We loop until empty because microtasks can schedule more microtasks.
        // Guard against infinite loops? u32 limit?
        let mut loop_count = 0;
        while !self.microtasks.is_empty() {
            self.microtasks.drain();
            loop_count += 1;
            if loop_count > 1000 {
                println!("Warn: Possible infinite microtask loop");
                break;
            }
        }

        // 2. Flush Effects
        self.effects.drain();

        // 3. Flush Layout Effects
        self.layout_effects.drain();

        *self.in_tick.borrow_mut() = false;

        !self.is_idle()
    }

    pub fn is_idle(&self) -> bool {
        self.microtasks.is_empty() && self.effects.is_empty() && self.layout_effects.is_empty()
    }
}

impl Scheduler for LocalScheduler {
    fn schedule_microtask(&self, task: Box<dyn FnOnce()>) {
        self.microtasks.push(task);
    }

    fn schedule_effect(&self, effect: Box<dyn FnOnce()>) {
        self.effects.push(effect);
    }

    fn schedule_layout_effect(&self, effect: Box<dyn FnOnce()>) {
        self.layout_effects.push(effect);
    }

    fn request_yield(&self) {
        // In a cooperative environment, this might just mean "return to host loop".
        // For LocalScheduler, it's a no-op as `tick` returns.
    }

    fn now(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64() * 1000.0
    }
}
