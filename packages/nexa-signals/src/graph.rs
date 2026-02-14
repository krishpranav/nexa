use slotmap::{SlotMap, new_key_type};
use smallvec::SmallVec;

new_key_type! {
    pub struct SignalId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Signal,
    Memo,
    Effect,
}

pub struct GraphNode {
    pub subscribers: SmallVec<[SignalId; 4]>,
    pub dependencies: SmallVec<[SignalId; 4]>,
    pub depth: u32,
    pub node_type: NodeType,
    pub update_fn: Option<std::rc::Rc<dyn Fn()>>,
}

pub struct Graph {
    pub nodes: SlotMap<SignalId, GraphNode>,
    pub dirty_nodes: std::collections::HashSet<SignalId>,
    pub batch_count: u32,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            dirty_nodes: std::collections::HashSet::new(),
            batch_count: 0,
        }
    }

    pub fn allocate(
        &mut self,
        node_type: NodeType,
        update_fn: Option<std::rc::Rc<dyn Fn()>>,
    ) -> SignalId {
        self.nodes.insert(GraphNode {
            subscribers: SmallVec::new(),
            dependencies: SmallVec::new(),
            depth: 0,
            node_type,
            update_fn,
        })
    }

    pub fn add_dependency(&mut self, subscriber: SignalId, dependency: SignalId) {
        // Prevent self-dependency
        if subscriber == dependency {
            panic!("Cyclic dependency detected: signal depends on itself");
        }

        // Check for cycles (simple DFS for now, can be optimized)
        if self.would_cause_cycle(subscriber, dependency) {
            panic!(
                "Cyclic dependency detected between {:?} and {:?}",
                subscriber, dependency
            );
        }

        let dep_depth = self.nodes.get(dependency).map(|n| n.depth).unwrap_or(0);

        if let Some(sub_node) = self.nodes.get_mut(subscriber) {
            if !sub_node.dependencies.contains(&dependency) {
                sub_node.dependencies.push(dependency);
                // Update depth to be max(dependencies) + 1
                sub_node.depth = sub_node.depth.max(dep_depth + 1);
            }
        }

        if let Some(dep_node) = self.nodes.get_mut(dependency) {
            if !dep_node.subscribers.contains(&subscriber) {
                dep_node.subscribers.push(subscriber);
            }
        }
    }

    fn would_cause_cycle(&self, subscriber: SignalId, dependency: SignalId) -> bool {
        // If dependency already depends on subscriber, adding subscriber -> dependency creates cycle
        let mut stack = vec![subscriber];
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = stack.pop() {
            if current == dependency {
                return true;
            }
            if !visited.insert(current) {
                continue;
            }
            if let Some(node) = self.nodes.get(current) {
                for &sub in &node.subscribers {
                    stack.push(sub);
                }
            }
        }
        false
    }

    pub fn mark_dirty(&mut self, id: SignalId) {
        self.dirty_nodes.insert(id);
    }

    pub fn take_dirty(&mut self) -> Vec<SignalId> {
        let mut dirty: Vec<_> = self.dirty_nodes.drain().collect();
        // Sort by depth to ensure correct propagation order
        dirty.sort_by_key(|&id| self.nodes.get(id).map(|n| n.depth).unwrap_or(0));
        dirty
    }
}
