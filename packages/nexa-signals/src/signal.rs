use crate::SignalId;
use crate::context::{allocate_node, mark_dirty, pop_observer, push_observer, track_read};
use crate::graph::NodeType;
use std::cell::UnsafeCell;
use std::rc::{Rc, Weak};

pub struct SignalInner<T> {
    pub value: UnsafeCell<T>,
}

pub struct Signal<T> {
    pub id: SignalId,
    pub inner: Rc<SignalInner<T>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            inner: self.inner.clone(),
        }
    }
}

impl<T: PartialEq> Signal<T> {
    pub fn new(value: T) -> Self {
        let id = allocate_node(NodeType::Signal, None);
        Self {
            id,
            inner: Rc::new(SignalInner {
                value: UnsafeCell::new(value),
            }),
        }
    }

    pub fn get(&self) -> &T {
        track_read(self.id);
        unsafe { &*self.inner.value.get() }
    }

    pub fn set(&self, new_value: T) {
        let same = unsafe { &*self.inner.value.get() == &new_value };
        if !same {
            unsafe {
                *self.inner.value.get() = new_value;
            }
            mark_dirty(self.id);
        }
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        track_read(self.id);
        let val = unsafe { &*self.inner.value.get() };
        f(val)
    }
}

pub struct MemoInner<T> {
    pub value: UnsafeCell<T>,
    pub compute_fn: Rc<dyn Fn() -> T>,
}

pub struct Memo<T> {
    pub id: SignalId,
    pub inner: Rc<MemoInner<T>>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            inner: self.inner.clone(),
        }
    }
}

impl<T: PartialEq + 'static> Memo<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let compute_fn = Rc::new(f);
        let inner = Rc::new(MemoInner {
            value: UnsafeCell::new(unsafe { std::mem::zeroed() }),
            compute_fn: compute_fn.clone(),
        });

        let id = allocate_node(NodeType::Memo, None);

        {
            let inner_weak = Rc::downgrade(&inner);
            let update_fn = Rc::new(move || {
                if let Some(inner) = inner_weak.upgrade() {
                    push_observer(id);
                    let new_val = (inner.compute_fn)();
                    pop_observer();

                    unsafe {
                        let old_val = &*inner.value.get();
                        if &new_val != old_val {
                            *inner.value.get() = new_val;
                            mark_dirty(id);
                        }
                    }
                }
            });

            // Initial compute
            (update_fn)();

            // Patch graph node
            crate::context::with_graph_mut(|g| {
                if let Some(node) = g.nodes.get_mut(id) {
                    node.update_fn = Some(update_fn);
                }
            });
        }

        Self { id, inner }
    }

    pub fn get(&self) -> &T {
        track_read(self.id);
        unsafe { &*self.inner.value.get() }
    }
}

pub struct Effect {
    pub id: SignalId,
}

impl Effect {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let run_fn = Rc::new(f);
        let id = allocate_node(NodeType::Effect, None);

        let run_fn_weak = Rc::downgrade(&run_fn);
        let update_fn = Rc::new(move || {
            if let Some(run_fn) = run_fn_weak.upgrade() {
                push_observer(id);
                (run_fn)();
                pop_observer();
            }
        });

        // Initial run
        (update_fn)();

        // Patch graph node
        crate::context::with_graph_mut(|g| {
            if let Some(node) = g.nodes.get_mut(id) {
                node.update_fn = Some(update_fn);
            }
        });

        Self { id }
    }
}

pub fn signal<T: PartialEq>(value: T) -> Signal<T> {
    Signal::new(value)
}
