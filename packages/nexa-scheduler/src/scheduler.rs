use nexa_signals::{NodeId, ReactiveNode, SignalGraph};
use smallvec::SmallVec;

pub struct Scheduler {
    pub dirty_queue: SmallVec<[NodeId; 16]>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            dirty_queue: SmallVec::new(),
        }
    }

    pub fn mark_dirty(&mut self, id: NodeId) {
        if !self.dirty_queue.contains(&id) {
            self.dirty_queue.push(id);
        }
    }

    pub fn run(&mut self, graph: &mut SignalGraph) {
        if self.dirty_queue.is_empty() {
            return;
        }

        // 1. Coalesce dirty nodes
        let mut processing_queue = self.dirty_queue.clone();
        self.dirty_queue.clear();

        // 2. Propagate dirty state to discover all affected nodes
        // (BFS to find all downstream subscribers)
        let mut i = 0;
        while i < processing_queue.len() {
            let id = processing_queue[i];
            i += 1;

            if let Some(node) = graph.nodes.get(id) {
                let subscribers = match node {
                    ReactiveNode::Signal(s) => &s.subscribers,
                    ReactiveNode::Computed(c) => &c.subscribers,
                    // Effects don't have subscribers usually
                    ReactiveNode::Effect(_) => continue,
                };

                for &sub in subscribers {
                    if !processing_queue.contains(&sub) {
                        processing_queue.push(sub);
                    }
                }
            }
        }

        // 3. Topological Sort (by Depth)
        // Since we maintained depth in graph insertion, we can just sort by depth.
        // Lower depth executes first? Actually, signals (depth 0) change, then computed (depth 1), etc.
        // Yes, stable sort by depth ascending.
        processing_queue.sort_by_key(|&id| match graph.nodes.get(id) {
            Some(ReactiveNode::Signal(_)) => 0,
            Some(ReactiveNode::Computed(c)) => c.depth,
            Some(ReactiveNode::Effect(e)) => e.depth,
            None => 0,
        });

        let nodes_to_run = processing_queue;

        // Step 4 — Batched Execution
        for node_id in nodes_to_run {
            // Need to borrow graph mutably to update values, but we also needed it immutably for sort.
            // Split bororws or just re-lookup.
            // In a real impl, we'd run the update function.
            // For now, this is the structural implementation.
            tracing::trace!("Running update for {:?}", node_id);
        }
    }
}
