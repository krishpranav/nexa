use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;

new_key_type! {
    pub struct SignalId;
}

pub struct GraphNode {
    pub subscribers: SmallVec<[SignalId; 4]>,
}

pub struct Graph {
    pub nodes: SlotMap<SignalId, GraphNode>,
    pub dirty_nodes: std::collections::HashSet<SignalId>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            dirty_nodes: std::collections::HashSet::new(),
        }
    }

    pub fn allocate(&mut self) -> SignalId {
        self.nodes.insert(GraphNode {
            subscribers: SmallVec::new(),
        })
    }

    pub fn add_dependency(&mut self, subscriber: SignalId, dependency: SignalId) {
        if let Some(node) = self.nodes.get_mut(dependency) {
            if !node.subscribers.contains(&subscriber) {
                node.subscribers.push(subscriber);
            }
        }
    }

    pub fn mark_dirty(&mut self, id: SignalId) {
        self.dirty_nodes.insert(id);
    }

    pub fn take_dirty(&mut self) -> Vec<SignalId> {
        let dirty: Vec<_> = self.dirty_nodes.drain().collect();
        dirty
    }
}
