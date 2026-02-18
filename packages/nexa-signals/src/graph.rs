use slotmap::{SlotMap, new_key_type};
use smallvec::SmallVec;
use std::collections::HashSet;
use std::rc::Rc;

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
    pub node_type: NodeType,

    // Dependencies (sources): Nodes that this node reads from
    pub dependencies: SmallVec<[SignalId; 4]>,

    // Subscribers (sinks): Nodes that read from this node
    pub subscribers: SmallVec<[SignalId; 4]>,

    // For sorting / cycle detection (optional, or just use during prop)
    pub depth: u32,

    // Dynamic update function (for Memos and Effects)
    // We use Weak to avoid cycles between Graph and Signal structs if they hold each other?
    // Actually Signal holds Rc<Inner>, Graph holds Closure capturing Weak<Inner>.
    pub update_fn: Option<Rc<dyn Fn()>>,
}

#[derive(Default)]
pub struct Graph {
    pub nodes: SlotMap<SignalId, GraphNode>,
    // Dirty nodes that need processing
    pub dirty_queue: HashSet<SignalId>,
    // Propagation epoch to avoid re-visiting or stale updates if needed
    pub epoch: u64,
    pub batch_depth: u32,
    pub in_propagation: bool,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            dirty_queue: HashSet::new(),
            epoch: 0,
            batch_depth: 0,
            in_propagation: false,
        }
    }

    pub fn allocate(&mut self, node_type: NodeType) -> SignalId {
        self.nodes.insert(GraphNode {
            node_type,
            dependencies: SmallVec::new(),
            subscribers: SmallVec::new(),
            depth: 0,
            update_fn: None,
        })
    }

    pub fn set_update_fn(&mut self, id: SignalId, f: Rc<dyn Fn()>) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.update_fn = Some(f);
        }
    }

    /// Clears all dependencies of a node (used before re-tracking)
    pub fn clear_dependencies(&mut self, dependent: SignalId) {
        // For each dependency, remove 'dependent' from its subscribers
        // We need to copy dependencies to avoid borrow conflict if we iterate and mutate
        // But we iterate `nodes.get(dep)` and mutate `dep`.

        // This is expensive O(N*M). M = deps.
        // Optimization: Use a temporary buffer or handle this carefully.

        let deps = if let Some(node) = self.nodes.get(dependent) {
            node.dependencies.clone()
        } else {
            return;
        };

        for dep_id in deps {
            if let Some(dep_node) = self.nodes.get_mut(dep_id) {
                if let Some(idx) = dep_node.subscribers.iter().position(|&s| s == dependent) {
                    dep_node.subscribers.swap_remove(idx);
                }
            }
        }

        if let Some(node) = self.nodes.get_mut(dependent) {
            node.dependencies.clear();
        }
    }

    pub fn add_dependency(&mut self, subscriber: SignalId, dependency: SignalId) {
        if subscriber == dependency {
            // Self-dependency: allowed (e.g. s.set(s.get() + 1))?
            // Usually no, for Memo it's a cycle.
            // For Signal, get() then set() is fine, but s depends on s? No.
            return;
        }

        if self.detect_cycle(subscriber, dependency) {
            panic!("Cyclic dependency detected");
        }

        let dep_depth = self.nodes.get(dependency).map(|n| n.depth).unwrap_or(0);

        if let Some(sub_node) = self.nodes.get_mut(subscriber) {
            if !sub_node.dependencies.contains(&dependency) {
                sub_node.dependencies.push(dependency);
                sub_node.depth = sub_node.depth.max(dep_depth + 1);
            }
        }

        if let Some(dep_node) = self.nodes.get_mut(dependency) {
            if !dep_node.subscribers.contains(&subscriber) {
                dep_node.subscribers.push(subscriber);
            }
        }
    }

    fn detect_cycle(&self, start: SignalId, target: SignalId) -> bool {
        // We want to add edge target -> start (target is dependency, start is subscriber).
        // Check if path start -> ... -> target exists.
        // BFS/DFS on subscribers.

        if start == target {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            if current == target {
                return true;
            }

            if !visited.insert(current) {
                continue;
            }

            if let Some(node) = self.nodes.get(current) {
                for &sub in &node.subscribers {
                    queue.push_back(sub);
                }
            }
        }

        false
    }

    pub fn remove_node(&mut self, id: SignalId) {
        self.clear_dependencies(id);
        // Also remove from deps?
        // Wait, clear_dependencies removes `id` from `sources.subscribers`.
        // We also need to remove `id` from `subscribers.dependencies`.

        // Copy subscribers
        let subs = if let Some(node) = self.nodes.get(id) {
            node.subscribers.clone()
        } else {
            SmallVec::new()
        };

        for sub_id in subs {
            if let Some(sub_node) = self.nodes.get_mut(sub_id) {
                if let Some(idx) = sub_node.dependencies.iter().position(|&d| d == id) {
                    sub_node.dependencies.swap_remove(idx);
                }
            }
        }

        self.nodes.remove(id);
    }
}
