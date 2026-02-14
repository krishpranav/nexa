use crate::SignalId;
use crate::graph::{Graph, NodeType};
use std::cell::RefCell;

thread_local! {
    pub static GRAPH: RefCell<Graph> = RefCell::new(Graph::new());
    pub static OBSERVERS: RefCell<Vec<SignalId>> = RefCell::new(Vec::new());
}

pub fn track_read(id: SignalId) {
    let observer = OBSERVERS.with(|o| o.borrow().last().copied());
    if let Some(observer) = observer {
        GRAPH.with(|g| {
            g.borrow_mut().add_dependency(observer, id);
        });
    }
}

pub fn mark_dirty(id: SignalId) {
    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        graph.mark_dirty(id);

        // If not in a batch, we could trigger immediate propagation
        // but for now we expect nexa-scheduler to take_dirty()
    });
}

pub fn allocate_node(node_type: NodeType, update_fn: Option<std::rc::Rc<dyn Fn()>>) -> SignalId {
    GRAPH.with(|g| g.borrow_mut().allocate(node_type, update_fn))
}

pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    GRAPH.with(|g| g.borrow_mut().batch_count += 1);
    let result = f();
    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        graph.batch_count -= 1;
        // Batch ended, but we still leave it to the scheduler to drain
    });
    result
}

pub fn push_observer(id: SignalId) {
    OBSERVERS.with(|o| o.borrow_mut().push(id));
}

pub fn pop_observer() {
    OBSERVERS.with(|o| o.borrow_mut().pop());
}

pub fn with_graph<F, R>(f: F) -> R
where
    F: FnOnce(&Graph) -> R,
{
    GRAPH.with(|g| f(&g.borrow()))
}

pub fn with_graph_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Graph) -> R,
{
    GRAPH.with(|g| f(&mut *g.borrow_mut()))
}

pub fn propagate(order: Vec<SignalId>) {
    let mut update_fns = Vec::new();

    GRAPH.with(|g| {
        let graph = g.borrow();
        for id in order {
            if let Some(node) = graph.nodes.get(id) {
                if let Some(update_fn) = &node.update_fn {
                    update_fns.push(update_fn.clone());
                }
            }
        }
    });

    for update_fn in update_fns {
        (update_fn)();
    }
}
