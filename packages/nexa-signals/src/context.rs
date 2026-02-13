use crate::graph::Graph;
use crate::SignalId;
use std::cell::RefCell;

thread_local! {
    pub static GRAPH: RefCell<Graph> = RefCell::new(Graph::new());
    pub static OBSERVER: RefCell<Option<SignalId>> = RefCell::new(None);
}

pub fn track_read(id: SignalId) {
    OBSERVER.with(|o| {
        if let Some(observer) = *o.borrow() {
            GRAPH.with(|g| {
                g.borrow_mut().add_dependency(observer, id);
            });
        }
    });
}

pub fn mark_dirty(id: SignalId) {
    GRAPH.with(|g| {
        g.borrow_mut().mark_dirty(id);
    });
}

pub fn allocate_node() -> SignalId {
    GRAPH.with(|g| g.borrow_mut().allocate())
}
