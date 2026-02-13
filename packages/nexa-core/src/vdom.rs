use rustc_hash::FxHashMap;
use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use std::borrow::Cow;

new_key_type! {
    pub struct NodeId;
}

pub struct VDomArena {
    pub nodes: SlotMap<NodeId, VNode>,
}

impl VDomArena {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, node: VNode) -> NodeId {
        self.nodes.insert(node)
    }

    pub fn get(&self, id: NodeId) -> Option<&VNode> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut VNode> {
        self.nodes.get_mut(id)
    }
}

pub enum VNode {
    Element(ElementNode),
    Text(TextNode),
    Fragment(FragmentNode),
    Component(ComponentNode),
    Placeholder,
}

pub struct ElementNode {
    pub tag: Cow<'static, str>,
    pub attributes: FxHashMap<Cow<'static, str>, String>,
    pub listeners: FxHashMap<Cow<'static, str>, NodeId>, // Listener ID or handler
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
}

pub struct TextNode {
    pub text: String,
    pub parent: Option<NodeId>,
}

pub struct FragmentNode {
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
}

pub struct ComponentNode {
    pub name: Cow<'static, str>,
    pub scope_id: Option<crate::runtime::ScopeId>, // Scope associated with this component
    pub props: Box<dyn std::any::Any>,             // Simplified props for now
    pub children: Option<NodeId>,                  // Rendered output
    pub parent: Option<NodeId>,
}
