use crate::SignalId;
use crate::dependency::{
    allocate_node, mark_subscribers_dirty, remove_node, set_update_fn, track_read, with_observer,
};
use crate::graph::NodeType;
use std::cell::UnsafeCell;
use std::rc::Rc;

pub struct SignalInner<T> {
    pub id: SignalId,
    pub value: UnsafeCell<T>,
}

impl<T> Drop for SignalInner<T> {
    fn drop(&mut self) {
        remove_node(self.id);
    }
}

pub struct Signal<T> {
    pub inner: Rc<SignalInner<T>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Signal<T> {
    pub fn id(&self) -> SignalId {
        self.inner.id
    }
}

impl<T: PartialEq + 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        let id = allocate_node(NodeType::Signal);
        Self {
            inner: Rc::new(SignalInner {
                id,
                value: UnsafeCell::new(value),
            }),
        }
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        track_read(self.inner.id);
        unsafe { (*self.inner.value.get()).clone() }
    }

    pub fn set(&self, new_value: T) {
        let same = unsafe { &*self.inner.value.get() == &new_value };
        if !same {
            unsafe {
                *self.inner.value.get() = new_value;
            }
            mark_subscribers_dirty(self.inner.id);
        }
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        track_read(self.inner.id);
        unsafe {
            f(&mut *self.inner.value.get());
        }
        mark_subscribers_dirty(self.inner.id);
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        track_read(self.inner.id);
        let val = unsafe { &*self.inner.value.get() };
        f(val)
    }
}

pub struct MemoInner<T> {
    pub id: SignalId,
    pub value: UnsafeCell<Option<T>>,
    pub compute_fn: Rc<dyn Fn() -> T>,
}

impl<T> Drop for MemoInner<T> {
    fn drop(&mut self) {
        remove_node(self.id);
    }
}

pub struct Memo<T> {
    pub inner: Rc<MemoInner<T>>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Memo<T> {
    pub fn id(&self) -> SignalId {
        self.inner.id
    }
}

impl<T: PartialEq + 'static> Memo<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let id = allocate_node(NodeType::Memo);
        let compute_fn = Rc::new(f);

        let inner = Rc::new(MemoInner {
            id,
            value: UnsafeCell::new(None),
            compute_fn: compute_fn.clone(),
        });

        {
            let inner_weak = Rc::downgrade(&inner);
            let update_fn = Rc::new(move || {
                if let Some(inner) = inner_weak.upgrade() {
                    let new_val = with_observer(id, || (inner.compute_fn)());

                    unsafe {
                        let val_ptr = inner.value.get();
                        if let Some(old_val) = &*val_ptr {
                            if old_val != &new_val {
                                *val_ptr = Some(new_val);
                                mark_subscribers_dirty(id);
                            }
                        } else {
                            // First run
                            *val_ptr = Some(new_val);
                            // No subscribers to notify on first run
                        }
                    }
                }
            });

            set_update_fn(id, update_fn.clone());

            // Run once to initialize and track deps
            (update_fn)();
        }

        Self { inner }
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        track_read(self.inner.id);
        unsafe {
            let val = &*self.inner.value.get();
            if let Some(v) = val {
                v.clone()
            } else {
                panic!("Memo not initialized");
            }
        }
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        track_read(self.inner.id);
        unsafe {
            let val = &*self.inner.value.get();
            if let Some(v) = val {
                f(v)
            } else {
                panic!("Memo not initialized");
            }
        }
    }
}

pub struct EffectInner {
    pub id: SignalId,
    pub run_fn: Rc<dyn Fn()>,
}

impl Drop for EffectInner {
    fn drop(&mut self) {
        crate::dependency::remove_node(self.id);
    }
}

pub struct Effect {
    pub inner: Rc<EffectInner>,
}

impl Effect {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + 'static,
    {
        let id = allocate_node(NodeType::Effect);
        let run_fn = Rc::new(f);

        let inner = Rc::new(EffectInner {
            id,
            run_fn: run_fn.clone(),
        });

        let inner_weak = Rc::downgrade(&inner);

        let update_fn = Rc::new(move || {
            if let Some(inner) = inner_weak.upgrade() {
                with_observer(id, || (inner.run_fn)());
            }
        });

        set_update_fn(id, update_fn.clone());

        // Initial run
        (update_fn)();

        Self { inner }
    }
}

pub fn signal<T: PartialEq + 'static>(value: T) -> Signal<T> {
    Signal::new(value)
}

pub fn create_memo<T: PartialEq + 'static, F: Fn() -> T + 'static>(f: F) -> Memo<T> {
    Memo::new(f)
}

pub fn create_effect<F: Fn() + 'static>(f: F) -> Effect {
    Effect::new(f)
}
