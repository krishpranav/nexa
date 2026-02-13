use slotmap::new_key_type;
use std::collections::HashMap;

new_key_type! {
    pub struct NodeId;
}

/// A node in the Virtual DOM.
#[derive(Debug, Clone)]
pub enum VNode {
    Element(VElement),
    Text(VText),
    Fragment(VFragment),
    Component(VComponent),
    Placeholder(VPlaceholder),
}

#[derive(Debug, Clone)]
pub struct VElement {
    pub tag: String,
    pub id: Option<NodeId>,                  // Mounted ID
    pub attributes: HashMap<String, String>, // Simplified for now
    pub children: Vec<NodeId>,
    pub listeners: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VText {
    pub text: String,
    pub id: Option<NodeId>,
}

#[derive(Debug, Clone)]
pub struct VFragment {
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct VComponent {
    pub name: String,
    // Props would go here, often generic or Any
    // scope_id: Option<ScopeId>,
}

#[derive(Debug, Clone)]
pub struct VPlaceholder {
    pub id: Option<NodeId>,
}

/// The Arena storing all VNodes.
pub struct VDomArena {
    pub nodes: slotmap::SlotMap<NodeId, VNode>,
}

impl VDomArena {
    pub fn new() -> Self {
        Self {
            nodes: slotmap::SlotMap::with_key(),
        }
    }

    pub fn alloc(&mut self, node: VNode) -> NodeId {
        self.nodes.insert(node)
    }

    pub fn get(&self, id: NodeId) -> Option<&VNode> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut VNode> {
        self.nodes.get_mut(id)
    }
}
