use std::cell::RefCell;
use std::collections::VecDeque;

/// A simple FIFO queue for tasks.
/// Since LocalScheduler is single-threaded, we use RefCell<VecDeque>.
#[derive(Default)]
pub struct TaskQueue {
    queue: RefCell<VecDeque<Box<dyn FnOnce()>>>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn push(&self, task: Box<dyn FnOnce()>) {
        self.queue.borrow_mut().push_back(task);
    }

    pub fn pop(&self) -> Option<Box<dyn FnOnce()>> {
        self.queue.borrow_mut().pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }

    pub fn drain(&self) {
        // We pop one by one to allow re-entrant scheduling?
        // Or we drain the whole buffer.
        // Usually draining is safer to avoid infinite loops in one tick if we put a limit.
        // But for now, let's just run until empty.
        while let Some(task) = self.pop() {
            task();
        }
    }
}
