use nexa_signals::{Graph, SignalId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

pub struct Scheduler {
    dirty_queue: Vec<SignalId>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            dirty_queue: Vec::new(),
        }
    }

    pub fn schedule(&mut self, dirty: impl IntoIterator<Item = SignalId>) {
        for id in dirty {
            // Avoid duplicates in queue?
            // For stability + perf, maybe check logic, but for now simple push.
            // Run() handles deductive logic.
            self.dirty_queue.push(id);
        }
    }

    pub fn run(&mut self, graph: &Graph) -> Vec<SignalId> {
        if self.dirty_queue.is_empty() {
            return Vec::new();
        }

        // 1. Discover reachable subgraph (Transitive Closure of Dirty Set)
        // We use a set to track visited/subgraph nodes for O(1) lookup.
        let mut subgraph_nodes = FxHashSet::default();
        let mut stack = Vec::new();

        // Seed with initial dirty queue
        for &id in &self.dirty_queue {
            if !subgraph_nodes.contains(&id) {
                subgraph_nodes.insert(id);
                stack.push(id);
            }
        }
        self.dirty_queue.clear();

        // Exploration to find all affected nodes
        let mut i = 0;
        while i < stack.len() {
            let u = stack[i];
            i += 1;

            if let Some(node) = graph.nodes.get(u) {
                for &v in &node.subscribers {
                    if !subgraph_nodes.contains(&v) {
                        subgraph_nodes.insert(v);
                        stack.push(v);
                    }
                }
            }
        }

        // `stack` now contains all nodes in the subgraph in BFS/Topo-ish discovery order.
        let nodes_to_process = stack;

        // 2. Compute In-Degrees associated *only* with edges within the subgraph
        let mut in_degrees = FxHashMap::default();

        // Initialize degrees
        for &id in &nodes_to_process {
            in_degrees.insert(id, 0);
        }

        for &u in &nodes_to_process {
            if let Some(node) = graph.nodes.get(u) {
                for &v in &node.subscribers {
                    if subgraph_nodes.contains(&v) {
                        *in_degrees.get_mut(&v).unwrap() += 1;
                    }
                }
            }
        }

        // 3. Kahn's Algorithm
        let mut queue = VecDeque::new();

        // Initialize queue with 0 in-degree nodes
        // Iterate over `nodes_to_process` to maintain deterministic discovery order
        // rather than iterating the HashMap.
        for &id in &nodes_to_process {
            if let Some(&deg) = in_degrees.get(&id) {
                if deg == 0 {
                    queue.push_back(id);
                }
            }
        }

        let mut sorted_order = Vec::with_capacity(nodes_to_process.len());

        while let Some(u) = queue.pop_front() {
            sorted_order.push(u);

            if let Some(node) = graph.nodes.get(u) {
                for &v in &node.subscribers {
                    // Only process edges within subgraph
                    if let Some(deg) = in_degrees.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }

        // If sorted_order.len() != nodes_to_process.len(), we have a cycle.
        // For now, we return what we have (or panic).
        // Since requirements imply deterministic output, we return result.

        sorted_order
    }
}
