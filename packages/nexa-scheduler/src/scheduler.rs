use nexa_signals::{Graph, NodeType, SignalId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityTier {
    Signal = 0,
    Effect = 1,
    Render = 2,
}

#[derive(Default, Debug, Clone)]
pub struct SchedulingStats {
    pub nodes_processed: u64,
    pub edges_traversed: u64,
    pub batch_count: u64,
}

/// A wrapper for SignalId to implement custom ordering in BinaryHeap
#[derive(PartialEq, Eq)]
struct ScheduledNode {
    id: SignalId,
    tier: PriorityTier,
    depth: u32,
}

impl Ord for ScheduledNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // We want a MIN-heap for tie-breaking by (Tier, Depth, Id)
        // BinaryHeap is a MAX-heap by default, so we reverse the comparisons
        other
            .tier
            .cmp(&self.tier)
            .then_with(|| other.depth.cmp(&self.depth))
            .then_with(|| other.id.cmp(&self.id))
    }
}

impl PartialOrd for ScheduledNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Scheduler {
    dirty_set: FxHashSet<SignalId>,
    pub stats: SchedulingStats,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            dirty_set: FxHashSet::default(),
            stats: SchedulingStats::default(),
        }
    }

    pub fn schedule(&mut self, dirty: impl IntoIterator<Item = SignalId>) {
        for id in dirty {
            self.dirty_set.insert(id);
        }
    }

    pub fn run(&mut self, graph: &Graph) -> Vec<SignalId> {
        if self.dirty_set.is_empty() {
            return Vec::new();
        }

        self.stats.batch_count += 1;

        // 1. Transitive Closure (Iterative discovery)
        let mut subgraph_nodes = FxHashSet::default();
        let mut stack: Vec<SignalId> = self.dirty_set.drain().collect();

        for &id in &stack {
            subgraph_nodes.insert(id);
        }

        let mut i = 0;
        while i < stack.len() {
            let u = stack[i];
            i += 1;
            self.stats.nodes_processed += 1;

            if let Some(node) = graph.nodes.get(u) {
                for &v in &node.subscribers {
                    self.stats.edges_traversed += 1;
                    if subgraph_nodes.insert(v) {
                        stack.push(v);
                    }
                }
            }
        }

        let nodes_to_process = stack;
        let mut in_degrees = FxHashMap::default();

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

        // 2. Stable Kahn's Algorithm using BinaryHeap
        let mut heap = BinaryHeap::new();

        for &id in &nodes_to_process {
            if let Some(&deg) = in_degrees.get(&id) {
                if deg == 0 {
                    if let Some(node) = graph.nodes.get(id) {
                        let tier = match node.node_type {
                            NodeType::Signal | NodeType::Memo => PriorityTier::Signal,
                            NodeType::Effect => PriorityTier::Effect,
                        };
                        heap.push(ScheduledNode {
                            id,
                            tier,
                            depth: node.depth,
                        });
                    }
                }
            }
        }

        let mut sorted_order = Vec::with_capacity(nodes_to_process.len());

        while let Some(ScheduledNode { id, .. }) = heap.pop() {
            sorted_order.push(id);

            if let Some(node) = graph.nodes.get(id) {
                for &v in &node.subscribers {
                    if let Some(deg) = in_degrees.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            if let Some(v_node) = graph.nodes.get(v) {
                                let tier = match v_node.node_type {
                                    NodeType::Signal | NodeType::Memo => PriorityTier::Signal,
                                    NodeType::Effect => PriorityTier::Effect,
                                };
                                heap.push(ScheduledNode {
                                    id: v,
                                    tier,
                                    depth: v_node.depth,
                                });
                            }
                        }
                    }
                }
            }
        }

        if sorted_order.len() != nodes_to_process.len() {
            panic!("Cycle detected in signals graph during scheduling!");
        }

        sorted_order
    }
}

impl nexa_core::Scheduler for Scheduler {
    fn schedule(&mut self, dirty: impl IntoIterator<Item = SignalId>) {
        self.schedule(dirty)
    }

    fn run(&mut self, graph: &Graph) -> Vec<SignalId> {
        self.run(graph)
    }
}
