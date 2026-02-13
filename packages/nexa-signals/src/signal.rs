use crate::context::{allocate_node, mark_dirty, track_read};
use crate::SignalId;
use std::cell::UnsafeCell;

pub struct Signal<T> {
    pub id: SignalId,
    pub value: UnsafeCell<T>,
}

impl<T> Signal<T> {
    pub fn new(value: T) -> Self {
        let id = allocate_node();
        Self {
            id,
            value: UnsafeCell::new(value),
        }
    }

    pub fn get(&self) -> &T {
        track_read(self.id);
        unsafe { &*self.value.get() }
    }

    pub fn set(&self, new_value: T) {
        unsafe {
            *self.value.get() = new_value;
        }
        mark_dirty(self.id);
    }

    // For convenience
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        track_read(self.id);
        let val = unsafe { &*self.value.get() };
        f(val)
    }

    pub fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let val = unsafe { &mut *self.value.get() };
        let res = f(val);
        mark_dirty(self.id);
        res
    }
}

pub fn signal<T>(value: T) -> Signal<T> {
    Signal::new(value)
}
