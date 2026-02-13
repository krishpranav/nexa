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
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
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
        // Queue to scheduler
        // Since Scheduler integration is later, we just print or no-op
        // tracing::trace!("Dirty: {:?}", id);

        // Naive propagation for now (if we had recursively dirty)
        if let Some(node) = self.nodes.get(id) {
            for &_sub in &node.subscribers {
                // mark_dirty(sub); // recurse
            }
        }
    }
}
