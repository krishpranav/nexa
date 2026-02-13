use nexa_signals::{NodeId, ReactiveNode, SignalGraph};
use std::collections::HashSet;
use std::collections::VecDeque;

pub struct Scheduler {
    dirty_queue: VecDeque<NodeId>,
    dirty_set: HashSet<NodeId>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            dirty_queue: VecDeque::new(),
            dirty_set: HashSet::new(),
        }
    }

    pub fn mark_dirty(&mut self, node: NodeId) {
        if self.dirty_set.insert(node) {
            self.dirty_queue.push_back(node);
        }
    }

    pub fn propagate(&mut self, graph: &SignalGraph) {
        // BFS propagation to find all dirty implementations
        // Note: In typical reactivity, we might just want to re-run what's dirty.
        // But if we need to propagate dirtiness to computed nodes that MIGHT change, we follow subscribers.
        // The PDF says: "Step 2 — Propagate Dirtiness... for sub in graph[node].subscribers... mark_dirty(sub)"

        // We use a temporary queue for propagation so we don't mix "root dirty" with "propagated dirty" if distinction matters,
        // but here it seems we just want to mark everything reachable as potentially dirty to be re-evaluated.
        // Actually, simpler: just use the existing queue.

        let mut i = 0;
        while i < self.dirty_queue.len() {
            let node_id = self.dirty_queue[i];
            i += 1;

            if let Some(node) = graph.nodes.get(node_id) {
                let subscribers = match node {
                    ReactiveNode::Signal(s) => &s.subscribers,
                    ReactiveNode::Computed(c) => &c.subscribers,
                    ReactiveNode::Effect(_) => continue, // Effects don't have subscribers usually
                };

                for &sub in subscribers {
                    if self.dirty_set.insert(sub) {
                        self.dirty_queue.push_back(sub);
                    }
                }
            }
        }
    }

    pub fn run(&mut self, graph: &mut SignalGraph) {
        self.propagate(graph);

        // Step 3 — Topological Sort by Depth
        // We take all dirty nodes, collect them, and sort.
        // We drain the queue or just convert to vector.
        let mut nodes_to_run: Vec<NodeId> = self.dirty_queue.drain(..).collect();
        self.dirty_set.clear();

        nodes_to_run.sort_by_key(|&id| {
            graph
                .nodes
                .get(id)
                .map(|n| match n {
                    ReactiveNode::Signal(_) => 0,
                    ReactiveNode::Computed(c) => c.depth,
                    ReactiveNode::Effect(e) => e.depth,
                })
                .unwrap_or(0)
        });

        // Step 4 — Batched Execution
        for _node_id in nodes_to_run {
            // Need to borrow graph mutably to update values, but we also needed it immutably for sort.
            // Split bororws or just re-lookup.
            // In a real impl, we'd run the update function.
            // Since we don't have the update closure stored in the node (it's typed storage elsewhere),
            // this loop defines the *order* of execution.
            // The PDF says: "recompute(node)" or "run_effect(node)".
            // For this first pass, we acknowledge the node.

            // Placeholder for actual execution logic:
            // let node = graph.nodes.get_mut(node_id); ...
        }
    }
}
