use crate::SignalId;
use crate::context::{allocate_node, mark_dirty, pop_observer, push_observer, track_read};
use crate::graph::NodeType;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub struct Signal<T> {
    pub id: SignalId,
    pub value: UnsafeCell<T>,
}

impl<T: PartialEq> Signal<T> {
    pub fn new(value: T) -> Self {
        let id = allocate_node(NodeType::Signal);
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
        let same = unsafe { &*self.value.get() == &new_value };
        if !same {
            unsafe {
                *self.value.get() = new_value;
            }
            mark_dirty(self.id);
        }
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        track_read(self.id);
        let val = unsafe { &*self.value.get() };
        f(val)
    }

    pub fn read_only(&self) -> ReadOnlySignal<'_, T> {
        ReadOnlySignal {
            id: self.id,
            value: unsafe { &*self.value.get() },
        }
    }
}

pub struct ReadOnlySignal<'a, T> {
    pub id: SignalId,
    value: &'a T,
}

impl<'a, T> ReadOnlySignal<'a, T> {
    pub fn get(&self) -> &T {
        track_read(self.id);
        self.value
    }
}

#[cfg(feature = "global-registry")]
use once_cell::sync::Lazy;
#[cfg(feature = "global-registry")]
use std::collections::HashMap;
#[cfg(feature = "global-registry")]
use std::sync::Mutex;

#[cfg(feature = "global-registry")]
static GLOBAL_REGISTRY: Lazy<Mutex<HashMap<String, SignalId>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(feature = "global-registry")]
pub fn register_global_signal(name: String, id: SignalId) {
    GLOBAL_REGISTRY.lock().unwrap().insert(name, id);
}

pub struct Memo<T> {
    pub id: SignalId,
    pub value: UnsafeCell<T>,
    compute_fn: Rc<dyn Fn() -> T>,
}

impl<T: PartialEq + 'static> Memo<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let id = allocate_node(NodeType::Memo);
        let compute_fn = Rc::new(f);

        // Initial compute
        push_observer(id);
        let val = (compute_fn)();
        pop_observer();

        Self {
            id,
            value: UnsafeCell::new(val),
            compute_fn,
        }
    }

    pub fn get(&self) -> &T {
        track_read(self.id);
        unsafe { &*self.value.get() }
    }

    pub fn update(&self) {
        push_observer(self.id);
        let new_val = (self.compute_fn)();
        pop_observer();

        let old_val = unsafe { &*self.value.get() };
        if &new_val != old_val {
            unsafe {
                *self.value.get() = new_val;
            }
            mark_dirty(self.id);
        }
    }
}

pub struct Effect {
    pub id: SignalId,
    run_fn: Rc<dyn Fn()>,
}

impl Effect {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let id = allocate_node(NodeType::Effect);
        let run_fn = Rc::new(f);

        push_observer(id);
        (run_fn)();
        pop_observer();

        Self { id, run_fn }
    }

    pub fn run(&self) {
        push_observer(self.id);
        (self.run_fn)();
        pop_observer();
    }
}

pub fn signal<T: PartialEq>(value: T) -> Signal<T> {
    Signal::new(value)
}
