use crate::SignalId;
use crate::graph::{Graph, NodeType};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    pub static GRAPH: RefCell<Graph> = RefCell::new(Graph::new());
    pub static OBSERVERS: RefCell<Vec<SignalId>> = RefCell::new(Vec::new());
}

pub fn track_read(id: SignalId) {
    let observer = OBSERVERS.with(|o| o.borrow().last().copied());
    if let Some(observer) = observer {
        GRAPH.with(|g| {
            // Adds dependency of 'observer' on 'id'
            // In graph terms: `observer` depends on `id`.
            // `id` adds `observer` to subscribers.
            g.borrow_mut().add_dependency(observer, id);
        });
    }
}

pub fn mark_dirty(id: SignalId) {
    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        // Skip if already dirty?
        if graph.dirty_queue.contains(&id) {
            return;
        }
        graph.dirty_queue.insert(id);

        // If batch depth > 0 or already propagating, we just leave it in dirty_queue.
        if graph.batch_depth == 0 && !graph.in_propagation {
            // Propagate
            drop(graph); // Drop borrow
            propagate();
        }
    });
}

pub fn take_dirty() -> Vec<SignalId> {
    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        let mut dirty: Vec<_> = graph.dirty_queue.drain().collect();
        // Sort by depth
        dirty.sort_by_key(|&id| graph.nodes.get(id).map(|n| n.depth).unwrap_or(0));
        dirty
    })
}

pub fn allocate_node(node_type: NodeType) -> SignalId {
    GRAPH.with(|g| g.borrow_mut().allocate(node_type))
}

pub fn set_update_fn(id: SignalId, f: Rc<dyn Fn()>) {
    GRAPH.with(|g| g.borrow_mut().set_update_fn(id, f));
}

pub fn clear_dependencies(id: SignalId) {
    GRAPH.with(|g| g.borrow_mut().clear_dependencies(id));
}

pub fn remove_node(id: SignalId) {
    GRAPH.with(|g| g.borrow_mut().remove_node(id));
}

pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    GRAPH.with(|g| g.borrow_mut().batch_depth += 1);
    let result = f();
    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        graph.batch_depth -= 1;
        if graph.batch_depth == 0 && !graph.dirty_queue.is_empty() {
            drop(graph);
            propagate();
        }
    });
    result
}

pub fn push_observer(id: SignalId) {
    OBSERVERS.with(|o| o.borrow_mut().push(id));
}

pub fn pop_observer() {
    OBSERVERS.with(|o| o.borrow_mut().pop());
}

pub fn with_observer<F, R>(id: SignalId, f: F) -> R
where
    F: FnOnce() -> R,
{
    // Before running a tracking scope (like Memo/Effect), we should clear old dependencies
    // Because we re-record them during execution.
    clear_dependencies(id);

    push_observer(id);
    let result = f();
    pop_observer();
    result
}

pub fn mark_subscribers_dirty(id: SignalId) {
    let subscribers = GRAPH.with(|g| {
        g.borrow()
            .nodes
            .get(id)
            .map(|n| n.subscribers.clone())
            .unwrap_or_default()
    });

    GRAPH.with(|g| {
        let mut graph = g.borrow_mut();
        for sub in subscribers {
            graph.dirty_queue.insert(sub);
        }

        if graph.batch_depth == 0 && !graph.in_propagation {
            drop(graph);
            propagate();
        }
    });
}

pub fn propagate() {
    GRAPH.with(|g| g.borrow_mut().in_propagation = true);

    // Basic propagation loop
    // 1. Take dirty nodes
    // 2. Topological sort (depth-based)
    // 3. Run updates
    // Note: We do NOT automatically add subscribers to dirty queue here.
    // The update_fn is responsible for calling mark_subscribers_dirty if value changed.

    loop {
        let dirty_batch = GRAPH.with(|g| {
            let mut graph = g.borrow_mut();
            if graph.dirty_queue.is_empty() {
                None
            } else {
                let mut dirty: Vec<_> = graph.dirty_queue.drain().collect();
                // Sort by depth
                dirty.sort_by_key(|&id| graph.nodes.get(id).map(|n| n.depth).unwrap_or(0));
                Some(dirty)
            }
        });

        if let Some(dirty_nodes) = dirty_batch {
            for id in dirty_nodes {
                let update_fn =
                    GRAPH.with(|g| g.borrow().nodes.get(id).and_then(|n| n.update_fn.clone()));

                if let Some(f) = update_fn {
                    f();
                }
            }
        } else {
            break;
        }
    }

    GRAPH.with(|g| g.borrow_mut().in_propagation = false);
}

pub fn with_graph<F, R>(f: F) -> R
where
    F: FnOnce(&Graph) -> R,
{
    GRAPH.with(|g| f(&g.borrow()))
}

pub fn execute(ids: Vec<SignalId>) {
    let mut update_fns = Vec::new();

    GRAPH.with(|g| {
        let graph = g.borrow();
        for id in ids {
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
