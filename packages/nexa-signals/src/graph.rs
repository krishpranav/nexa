use crate::nodes::{ComputedNode, EffectNode, NodeId, ReactiveNode, SignalNode};
use slotmap::SlotMap;
use smallvec::SmallVec;

pub struct SignalGraph {
    pub nodes: SlotMap<NodeId, ReactiveNode>,
    // Value storage would be here or external.
    // Implementing purely graph topology for now as per "Reactive graph" requirement.
}

impl SignalGraph {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
        }
    }

    pub fn insert_signal(&mut self) -> NodeId {
        self.nodes.insert(ReactiveNode::Signal(SignalNode {
            subscribers: SmallVec::new(),
        }))
    }

    pub fn insert_computed(&mut self, dependencies: SmallVec<[NodeId; 4]>) -> NodeId {
        let mut depth = 0;
        for &dep in &dependencies {
            if let Some(node) = self.nodes.get(dep) {
                let d = match node {
                    ReactiveNode::Signal(_) => 0,
                    ReactiveNode::Computed(c) => c.depth,
                    ReactiveNode::Effect(e) => e.depth, // Effects usually don't have dependents so 0?
                };
                depth = std::cmp::max(depth, d);
            }
        }

        // Depth = max(deps) + 1
        let depth = depth + 1;

        let id = self.nodes.insert(ReactiveNode::Computed(ComputedNode {
            dependencies: dependencies.clone(),
            subscribers: SmallVec::new(),
            depth,
        }));

        // Link dependencies to this node
        for &dep in &dependencies {
            if let Some(node) = self.nodes.get_mut(dep) {
                match node {
                    ReactiveNode::Signal(s) => s.subscribers.push(id),
                    ReactiveNode::Computed(c) => c.subscribers.push(id),
                    _ => {}
                }
            }
        }
        id
    }

    pub fn insert_effect(&mut self, dependencies: SmallVec<[NodeId; 4]>) -> NodeId {
        let mut depth = 0;
        for &dep in &dependencies {
            if let Some(node) = self.nodes.get(dep) {
                let d = match node {
                    ReactiveNode::Signal(_) => 0,
                    ReactiveNode::Computed(c) => c.depth,
                    ReactiveNode::Effect(e) => e.depth,
                };
                depth = std::cmp::max(depth, d);
            }
        }
        let depth = depth + 1;

        let id = self.nodes.insert(ReactiveNode::Effect(EffectNode {
            dependencies: dependencies.clone(),
            depth,
        }));

        for &dep in &dependencies {
            if let Some(node) = self.nodes.get_mut(dep) {
                match node {
                    ReactiveNode::Signal(s) => s.subscribers.push(id),
                    ReactiveNode::Computed(c) => c.subscribers.push(id),
                    _ => {}
                }
            }
        }
        id
    }
}
