use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;

new_key_type! {
    pub struct NodeId;
}

#[derive(Default)]
pub struct GenericArena<T> {
    items: SlotMap<NodeId, T>,
}

impl<T> GenericArena<T> {
    pub fn new() -> Self {
        Self {
            items: SlotMap::with_key(),
        }
    }
    pub fn insert(&mut self, item: T) -> NodeId {
        self.items.insert(item)
    }
    pub fn get(&self, id: NodeId) -> Option<&T> {
        self.items.get(id)
    }
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.items.get_mut(id)
    }
}

pub struct VDomArena {
    pub nodes: GenericArena<VirtualNode>,
}

impl VDomArena {
    pub fn new() -> Self {
        Self {
            nodes: GenericArena::new(),
        }
    }
}

pub enum VirtualNode {
    Element(Element),
    Text(Text),
    Fragment(Fragment),
    Component(Component),
    Placeholder,
}

pub struct Element {
    pub tag: &'static str,
    pub props: SmallVec<[Attribute; 4]>,
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
}

pub struct Attribute {
    pub name: &'static str,
    pub value: String, // Simplified for now, could be Any
}

pub struct Text {
    pub text: String,
    pub parent: Option<NodeId>,
}

pub struct Fragment {
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
}

pub struct Component {
    pub name: &'static str,
    pub render_fn: fn() -> NodeId, // Placeholder for component function
    pub scope: Option<crate::runtime::ScopeId>,
    pub parent: Option<NodeId>,
}
